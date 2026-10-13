package io.odalnode.oracle

import com.nimbusds.jose.JOSEException
import com.nimbusds.jose.JWSAlgorithm
import com.nimbusds.jose.JWSHeader
import com.nimbusds.jose.JWSVerifier
import com.nimbusds.jose.crypto.ECDSAVerifier
import com.nimbusds.jose.jca.JCAContext
import com.nimbusds.jose.jwk.ECKey
import com.nimbusds.jose.util.Base64URL
import com.nimbusds.jwt.SignedJWT
import eu.europa.ec.eudi.sdjwt.KeyBindingVerifier
import eu.europa.ec.eudi.sdjwt.NimbusSdJwtOps
import eu.europa.ec.eudi.sdjwt.SdJwt
import eu.europa.ec.eudi.sdjwt.SdJwtRecreateClaimsOps
import eu.europa.ec.eudi.sdjwt.SdJwtVerificationException
import kotlinx.coroutines.runBlocking
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.boolean
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import java.io.File
import java.math.BigDecimal
import java.security.KeyFactory
import java.security.KeyPairGenerator
import java.security.PublicKey
import java.security.Signature
import java.security.SignatureException
import java.security.spec.X509EncodedKeySpec
import java.util.Base64
import kotlin.system.exitProcess

/**
 * Judge the SD-JWTs this workspace emits with the EU digital identity wallet's
 * Kotlin SD-JWT library, which shares no code, author or crypto stack with this
 * crate or with the TypeScript library the first oracle uses.
 *
 * Reads the same `corpus.json` as `.github/oracle/sdjwt/judge.mjs`. The library
 * takes its signature check as a function, so Ed25519 is verified here with the
 * JDK's own provider (JDK 15 and later): a third implementation of the signature,
 * besides OpenSSL and `ed25519-dalek`.
 */

/** What the library made of a token. */
sealed interface Outcome {
    data class Accepted(val claims: JsonObject) : Outcome
    data class Refused(val why: String) : Outcome
}

/** Ed25519 through the JDK's provider, offered to Nimbus as a JWS verifier. */
class JdkEd25519Verifier(rawPublicKey: ByteArray) : JWSVerifier {
    private val key: PublicKey = KeyFactory.getInstance("Ed25519")
        .generatePublic(X509EncodedKeySpec(SPKI_PREFIX + rawPublicKey))
    private val context = JCAContext()

    override fun supportedJWSAlgorithms(): Set<JWSAlgorithm> = setOf(JWSAlgorithm.EdDSA)

    override fun getJCAContext(): JCAContext = context

    override fun verify(header: JWSHeader, signingInput: ByteArray, signature: Base64URL): Boolean {
        if (header.algorithm != JWSAlgorithm.EdDSA) throw JOSEException("not an EdDSA token")
        val verifier = Signature.getInstance("Ed25519")
        verifier.initVerify(key)
        verifier.update(signingInput)
        return try {
            verifier.verify(signature.decode())
        } catch (_: SignatureException) {
            false
        }
    }

    private companion object {
        /** DER prefix of an Ed25519 SubjectPublicKeyInfo, RFC 8410 section 4. */
        val SPKI_PREFIX = "302a300506032b6570032100".chunked(2).map { it.toInt(16).toByte() }.toByteArray()
    }
}

private val json = Json { isLenient = false }
private val b64url = Base64.getUrlDecoder()

/** RFC 9901 Appendix A.5, verbatim: the key that validates the Issuer signatures. */
private const val RFC_ISSUER_KEY =
    """{"kty":"EC","crv":"P-256","x":"b28d4MwZMjw8-00CG4xfnn9SLMVMM19SlqZpVb_uNtQ","y":"Xv5zWwuoaTgdS6hV43yI6gBwTnjukmFQQnJ_kCxzqk8"}"""

private fun describe(e: Throwable): String = when (e) {
    is SdJwtVerificationException -> e.reason.let { it::class.simpleName ?: it.toString() }
    else -> "${e::class.simpleName}: ${e.message}".take(160)
}

/** The claims clause 7.1 step 3 recreates from a verified SD-JWT. */
private fun recreate(sdJwt: SdJwt<SignedJWT>): Outcome {
    val payload = json.parseToJsonElement(sdJwt.jwt.payload.toString()).jsonObject
    return SdJwtRecreateClaimsOps.recreateClaimsAndDisclosuresPerClaim(payload, sdJwt.disclosures).fold(
        onSuccess = { (claims, _) -> Outcome.Accepted(claims) },
        onFailure = { Outcome.Refused(describe(it)) },
    )
}

/**
 * Verify with `verifier`, then recreate the claims the way clause 7.1 describes.
 *
 * This library has two modes and no third: a Key Binding JWT must be absent, or it
 * must be present and valid. So a token that carries one is verified in the second,
 * with the holder key from the `cnf` claim and the nonce and audience the Key
 * Binding JWT itself names, which is what the first oracle's judge does too.
 *
 * Never throws. This judge exists to be handed tokens built to break a reader, and
 * what a library does with one may be anything, a stack overflow on deep nesting
 * included. Whatever it is, it is a refusal to report, not a reason to stop the run
 * and lose the verdicts after it. That is why the catch below is as wide as it is.
 */
@Suppress("TooGenericExceptionCaught")
fun judge(token: String, verifier: JWSVerifier): Outcome = runBlocking {
    try {
        with(NimbusSdJwtOps) {
            val signature = verifier.asJwtVerifier()
            val keyBinding = token.substringAfterLast('~')
            if (keyBinding.isEmpty()) {
                verify(signature, token).fold(
                    onSuccess = { recreate(it) },
                    onFailure = { Outcome.Refused(describe(it)) },
                )
            } else {
                val claims = json.parseToJsonElement(String(b64url.decode(keyBinding.split('.')[1]))).jsonObject
                val challenge = JsonObject(mapOf("nonce" to claims.getValue("nonce"), "aud" to claims.getValue("aud")))
                val keyBindingVerifier = KeyBindingVerifier.mustBePresentAndValid(challenge = challenge)
                verify(signature, keyBindingVerifier, null, token).fold(
                    onSuccess = { recreate(it.sdJwt) },
                    onFailure = { Outcome.Refused(describe(it)) },
                )
            }
        }
    } catch (e: Throwable) {
        Outcome.Refused(describe(e))
    }
}

/** Key order is not meaning in JSON, and `12.0` is `12`. */
fun same(a: JsonElement, b: JsonElement): Boolean = when {
    a is JsonObject && b is JsonObject -> a.keys == b.keys && a.keys.all { same(a.getValue(it), b.getValue(it)) }
    a is JsonArray && b is JsonArray -> a.size == b.size && a.indices.all { same(a[it], b[it]) }
    a is JsonNull && b is JsonNull -> true
    a is JsonPrimitive && b is JsonPrimitive -> when {
        a.isString || b.isString -> a.isString && b.isString && a.content == b.content
        else -> {
            val x = a.content.toBigDecimalOrNull()
            val y = b.content.toBigDecimalOrNull()
            if (x != null && y != null) x.compareTo(y) == 0 else a.content == b.content
        }
    }
    else -> false
}

private fun String.toBigDecimalOrNull(): BigDecimal? = try { BigDecimal(this) } catch (_: NumberFormatException) { null }

fun main(args: Array<String>) {
    val path = args.firstOrNull() ?: error("usage: SdJwtJvmOracleKt <corpus.json>")
    val corpus = json.parseToJsonElement(File(path).readText()).jsonObject
    val cases = corpus.getValue("cases").jsonArray.map { it.jsonObject }
    val issuerKey = b64url.decode(corpus.getValue("issuerJwk").jsonObject.getValue("x").jsonPrimitive.content)
    val ed25519 = JdkEd25519Verifier(issuerKey)
    val es256 = ECDSAVerifier(ECKey.parse(RFC_ISSUER_KEY))

    println("independent implementation: eudi-lib-jvm-sdjwt-kt (Nimbus JOSE), signature: JDK Ed25519 / Nimbus ES256")
    println("java ${System.getProperty("java.version")} (${System.getProperty("java.vendor")}), ${cases.size} cases in $path")

    // Negative controls first: a judge that cannot refuse proves nothing.
    // Any compact token this judge accepts will do: the corpus has a control for it,
    // a credentials file has issued credentials.
    val valid = cases.filter { it["group"]?.jsonPrimitive?.content != "rfc" }
        .map { it.getValue("token").jsonPrimitive.content }
        .first { it.substringAfterLast('~').isEmpty() && judge(it, ed25519) is Outcome.Accepted }
    val problems = mutableListOf<String>()
    if (judge(valid, ed25519) !is Outcome.Accepted) problems += "control/valid is refused"
    val (jwt, rest) = valid.substringBefore('~') to valid.substringAfter('~')
    val parts = jwt.split('.')
    val flipped = listOf(parts[0], parts[1], (if (parts[2][0] == 'A') "B" else "A") + parts[2].drop(1)).joinToString(".") + "~" + rest
    if (judge(flipped, ed25519) is Outcome.Accepted) problems += "a changed signature was accepted"
    // A real key that did not sign it: arbitrary bytes need not be a point on the curve.
    val stranger = JdkEd25519Verifier(
        KeyPairGenerator.getInstance("Ed25519").generateKeyPair().public.encoded.takeLast(32).toByteArray(),
    )
    if (judge(valid, stranger) is Outcome.Accepted) problems += "a stranger's key was accepted"
    if (problems.isNotEmpty()) {
        System.err.println("NEGATIVE CONTROLS FAILED: $problems")
        exitProcess(2)
    }
    println("negative controls: the judge refuses what it must")

    val failures = mutableListOf<String>()
    val counts = linkedMapOf<String, Int>()
    val disagreements = mutableListOf<String>()
    val listed = linkedSetOf<String>()

    for (case in cases) {
        val id = case.getValue("id").jsonPrimitive.content
        // `credentials.json` has no groups: every case in it is a credential.
        val group = case["group"]?.jsonPrimitive?.content ?: "credential"
        val token = case.getValue("token").jsonPrimitive.content
        counts.merge(group, 1, Int::plus)
        val outcome = judge(token, if (group == "rfc") es256 else ed25519)
        when (group) {
            "issued", "presented" -> {
                val expected = case.getValue("expected")
                when (outcome) {
                    is Outcome.Refused -> failures += "$id: refused (${outcome.why})"
                    is Outcome.Accepted -> if (!same(outcome.claims, expected)) failures += "$id: read different claims"
                }
            }
            "adversarial" -> {
                val ours = case.getValue("ours").jsonObject
                val oursAccepts = ours.getValue("accepts").jsonPrimitive.boolean
                val line = when (outcome) {
                    is Outcome.Accepted ->
                        if (!oursAccepts) "$id: this crate refuses, the library accepts and reads ${outcome.claims.toString().take(150)}"
                        else if (!same(outcome.claims, ours.getValue("payload"))) "$id: both accept, different claims"
                        else null
                    is Outcome.Refused ->
                        if (oursAccepts) "$id: this crate accepts, the library refuses (${outcome.why})" else null
                }
                val known = KNOWN[id]
                when {
                    line != null && known == null -> disagreements += line
                    line != null && known != null -> {
                        listed += id
                        if (known.oursAccepts != oursAccepts) failures += "$id: the listed difference is the other way round now"
                    }
                    line == null && known != null -> failures += "$id: listed as a difference, but the two now agree: remove the entry"
                }
            }
            "credential" -> when (outcome) {
                is Outcome.Refused -> failures += "$id: refused (${outcome.why})"
                is Outcome.Accepted -> {
                    val ours = case.getValue("ours").jsonObject
                    val expected = case["expected"]
                    if (expected != null && !same(outcome.claims, expected)) {
                        failures += "$id: does not read the passport it was issued from"
                    } else if (ours.getValue("accepts").jsonPrimitive.boolean && !same(outcome.claims, ours.getValue("payload"))) {
                        failures += "$id: this crate and the library read different claims"
                    }
                }
            }
            "rfc" -> when (outcome) {
                is Outcome.Refused -> failures += "$id: the library does not verify the RFC's own token (${outcome.why})"
                is Outcome.Accepted -> {
                    val printed = case["printed"]
                    val ours = case.getValue("ours").jsonObject
                    if (printed != null && printed !is JsonNull && !same(outcome.claims, printed)) {
                        failures += "$id: differs from the payload the RFC prints"
                    }
                    // Two of the seven are printed issued and never processed, so the
                    // RFC gives nothing to compare with; this crate's reading is.
                    if (!ours.getValue("accepts").jsonPrimitive.boolean) {
                        failures += "$id: this crate refuses it (${ours["error"]})"
                    } else if (!same(outcome.claims, ours.getValue("payload"))) {
                        failures += "$id: this crate and the library read different claims"
                    }
                }
            }
        }
    }

    for (id in KNOWN.keys) {
        if (cases.none { it.getValue("id").jsonPrimitive.content == id } && cases.any { it["group"] != null }) {
            failures += "$id: listed as a difference, but the corpus has no such case"
        }
    }

    println(counts.entries.joinToString(", ") { "${it.key}: ${it.value}" })
    println("\nknown differences, each argued in the source: ${listed.size}")
    for (id in listed) {
        val k = KNOWN.getValue(id)
        println("  $id\n    this crate ${if (k.oursAccepts) "accepts" else "refuses"}; RFC supports ${k.supports}. ${k.reason}")
    }
    println("\nunexplained differences from this crate: ${disagreements.size}")
    disagreements.forEach { println("  $it") }
    println("\nfailures: ${failures.size}")
    failures.take(40).forEach { println("  $it") }
    if (failures.size > 40) println("  ... and ${failures.size - 40} more")
    if (failures.isNotEmpty() || disagreements.isNotEmpty()) exitProcess(1)
    println("every case agrees, or is a listed difference")
}

/** A difference from this crate that is not a defect in it, with the clause and the side the RFC supports. */
private data class Known(val oursAccepts: Boolean, val supports: String, val reason: String)

private val KNOWN = mapOf(
    "adversarial/control/key-binding-segment-present" to Known(
        true,
        "both",
        "The RFC reads two ways for a Verifier that does not require Key Binding and is handed an SD-JWT+KB. Clause 4: " +
            "a Verifier that expects an SD-JWT MUST verify that the last segment is empty, which is the library's " +
            "refusal (it has two modes, a Key Binding JWT absent, or present and valid). Clause 7.3 step 3 parses an " +
            "SD-JWT+KB whatever the policy and step 5 checks it only when required, which is how this crate and the " +
            "first oracle read it. This crate cannot know which its caller is, so it carries the segment and leaves " +
            "the refusal to a caller that expects none (`has_key_binding`). The control's Key Binding JWT is signed " +
            "by the issuer's key and names no `cnf`, so there is nothing to check it against.",
    ),
    "adversarial/reject/sd-alg-nested-inside-an-object" to Known(
        false,
        "neither",
        "Clause 4.1.1 forbids an Issuer to put `_sd_alg` below the top level, and clause 7.1 gives a Verifier no " +
            "rejection step for it. The library reads it as an ordinary claim and keeps it. This crate and the first " +
            "oracle refuse it, which is stricter than the clause requires.",
    ),
    "adversarial/reject/disclosure-salt-is-a-number" to Known(
        false,
        "this crate",
        "Clause 7.1 step 3.c.ii.1: the SD-JWT MUST be rejected unless the Disclosure is a JSON array of three " +
            "elements (salt, claim name, claim value), and clause 4.2.1 says the salt MUST be a string. The library " +
            "accepts a number. This crate and the first oracle refuse it. The library is the lenient one, reported " +
            "upstream as eudi-lib-jvm-sdjwt-kt#520.",
    ),
    "adversarial/reject/disclosure-name-is-a-number" to Known(
        false,
        "this crate",
        "Clause 7.1 step 3.c.ii.1, with clause 4.2.1's claim name, which MUST be a string; step 3.c.ii.4 then " +
            "inserts a claim under that name, and a JSON member name is a string (RFC 8259 section 4). The library " +
            "accepts a number and reads the claim as \"5\", a name the Issuer never wrote. This crate and the first " +
            "oracle refuse it. The library is the lenient one, reported upstream as eudi-lib-jvm-sdjwt-kt#520.",
    ),
    "adversarial/reject/array-element-disclosure-salt-is-a-number" to Known(
        false,
        "this crate",
        "Clause 7.1 step 3.c.iii.1: the SD-JWT MUST be rejected unless the Disclosure is a JSON array of two " +
            "elements (salt, value), with clause 4.2.1's salt, a string. The library accepts a number on this path " +
            "too. This crate and the first oracle refuse it. The library is the lenient one, as reported upstream.",
    ),
    "adversarial/edge/hash-algorithm-sha-512" to Known(
        false,
        "both",
        "Clause 4.1.1 allows any IANA hash and clause 7.1 step 2.d leaves the choice to the verifier's policy. " +
            "This crate implements SHA-256 only, on purpose.",
    ),
    "adversarial/edge/nested-200-deep" to Known(
        false,
        "both",
        "No clause limits nesting. serde_json refuses a document nested more than 127 deep, and a receiver sets " +
            "its own limit.",
    ),
)
