package io.odalnode.oracle

import com.nimbusds.jose.JWSVerifier
import com.nimbusds.jose.crypto.ECDSAVerifier
import com.nimbusds.jose.jwk.ECKey
import com.nimbusds.jose.jwk.OctetKeyPair
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.boolean
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.long
import java.io.File
import java.util.Base64
import kotlin.system.exitProcess

/**
 * Judge the `vc+sd-jwt` and `vp+sd-jwt` tokens `dpp_vc::jose` issues, with the EU digital
 * identity wallet's Kotlin SD-JWT library: the same corpus as
 * `.github/oracle/sdjwt/vc-sd-jwt.mjs`, which says how it is built
 * (`EMIT_VC_SD_JWT_CORPUS=1 cargo test -p dpp-vc --lib jose::tests::sd_jwt::oracle`).
 *
 * What it checks for each token: the signature under the published key (Nimbus: Ed25519 on
 * Tink, and ECDSA for P-256, P-384 and P-521), the Disclosures and their digests, which the
 * library processes and refuses if one matches nothing, `typ` and `cty` from the header
 * (the library knows nothing of the VC-JOSE-COSE media types), and the processed claims
 * against the claims this crate reads and against the document the token was made from,
 * with the concealed claims that were not presented taken out. The TypeScript judge also
 * recomputes each digest itself; this one leaves that to the library.
 *
 * A case with a `challenge` is a presentation bound to the Holder's key, which the signer
 * wrote as `cnf`. It is judged with Key Binding required, at that challenge, through
 * [judgeBound]: the library's own `KeyBindingVerifier.mustBePresentAndValid` takes the Holder
 * key from `cnf` and checks the Key Binding JWT, and `verify` checks `sd_hash` and the `iat`
 * window. The case's `accepts` is the verdict every reader must reach: the controls are
 * accepted, and the negative controls (a wrong nonce, a wrong audience, an `sd_hash` that
 * belongs to another presentation, and no Key Binding JWT) must be refused.
 *
 * Every check is the library's or Nimbus's own, and the judge holds no cryptography.
 * Reports every failure and exits non-zero if there is one.
 */

private val corpusJson = Json { isLenient = false }
private val base64Url = Base64.getUrlDecoder()

/** Nimbus's verifier for the public key in `jwk`: Ed25519 (`OKP`), or ECDSA (`EC`, any of P-256, P-384, P-521). */
private fun verifierFor(jwk: JsonObject): JWSVerifier =
    if (jwk.getValue("kty").jsonPrimitive.content == "OKP") {
        ed25519(OctetKeyPair.parse(jwk.toString()))
    } else {
        ECDSAVerifier(ECKey.parse(jwk.toString()))
    }

/** The protected header of the Issuer-signed JWT of an SD-JWT, read without checking anything. */
private fun headerOf(token: String): JsonObject =
    corpusJson.parseToJsonElement(String(base64Url.decode(token.substringBefore('~').split('.')[0]))).jsonObject

fun main(args: Array<String>) {
    val path = args.firstOrNull() ?: error("usage: VcSdJwtOracleKt <vc-sd-jwt.json>")
    val cases = corpusJson.parseToJsonElement(File(path).readText()).jsonObject.getValue("cases").jsonArray.map { it.jsonObject }

    println("independent implementation: eudi-lib-jvm-sdjwt-kt (Nimbus JOSE), signature: Nimbus Ed25519 (Tink) / Nimbus ECDSA")
    println("java ${System.getProperty("java.version")} (${System.getProperty("java.vendor")}), ${cases.size} tokens in $path")

    // Negative control first: a judge that cannot refuse proves nothing. A changed
    // signature must be refused, for a token this judge accepts.
    val control = cases.first { it.getValue("id").jsonPrimitive.content == "suite/row-17/issued" }
    val controlToken = control.getValue("token").jsonPrimitive.content
    val controlVerifier = verifierFor(control.getValue("jwk").jsonObject)
    val problems = mutableListOf<String>()
    if (judge(controlToken, controlVerifier) !is Outcome.Accepted) problems += "control/valid is refused"
    val (jwt, rest) = controlToken.substringBefore('~') to controlToken.substringAfter('~')
    val parts = jwt.split('.')
    val flipped = listOf(parts[0], parts[1], (if (parts[2][0] == 'A') "B" else "A") + parts[2].drop(1)).joinToString(".") + "~" + rest
    if (judge(flipped, controlVerifier) is Outcome.Accepted) problems += "a changed signature was accepted"
    if (problems.isNotEmpty()) {
        System.err.println("NEGATIVE CONTROLS FAILED: $problems")
        exitProcess(2)
    }
    println("negative controls: the judge refuses what it must")

    val failures = mutableListOf<String>()
    val algs = sortedSetOf<String>()
    for (case in cases) {
        val id = case.getValue("id").jsonPrimitive.content
        val token = case.getValue("token").jsonPrimitive.content
        val alg = case.getValue("alg").jsonPrimitive.content
        algs += alg

        // The challenge, for a case bound to a Holder and judged with Key Binding required.
        val challenge = case["challenge"]?.jsonObject?.let {
            Challenge(
                it.getValue("nonce").jsonPrimitive.content,
                it.getValue("aud").jsonPrimitive.content,
                it.getValue("now").jsonPrimitive.long,
                it.getValue("window").jsonPrimitive.long,
            )
        }
        val mustAccept = case["accepts"]?.jsonPrimitive?.boolean ?: true

        // A token this crate issues to no Holder carries no Key Binding JWT: the last segment is empty.
        if (challenge == null && token.substringAfterLast('~').isNotEmpty()) failures += "$id: the token carries a Key Binding JWT"

        val header = headerOf(token)
        fun headerIs(member: String, expected: String) {
            val found = header[member]?.jsonPrimitive?.content
            if (found != expected) failures += "$id: $member is $found, expected $expected"
        }
        headerIs("typ", case.getValue("typ").jsonPrimitive.content)
        headerIs("cty", case.getValue("cty").jsonPrimitive.content)
        headerIs("alg", alg)
        headerIs("kid", case.getValue("kid").jsonPrimitive.content)
        val kind = case.getValue("kind").jsonPrimitive.content
        headerIs("typ", if (kind == "credential") "vc+sd-jwt" else "vp+sd-jwt")
        headerIs("cty", if (kind == "credential") "vc" else "vp")

        val verifier = verifierFor(case.getValue("jwk").jsonObject)
        val judged = if (challenge == null) judge(token, verifier) else judgeBound(token, verifier, challenge)
        val ours = case.getValue("ours").jsonObject
        if (!mustAccept) {
            // A negative control: the library and this crate must both refuse it.
            if (judged is Outcome.Accepted) failures += "$id: the library accepts a token that must be refused"
            if (ours.getValue("accepts").jsonPrimitive.boolean) failures += "$id: this crate accepts a token that must be refused"
            continue
        }
        when (val outcome = judged) {
            is Outcome.Refused -> failures += "$id: refused (${outcome.why})"
            is Outcome.Accepted -> {
                if (!ours.getValue("accepts").jsonPrimitive.boolean) {
                    failures += "$id: this crate refuses its own token (${ours["error"]})"
                } else if (!same(outcome.claims, ours.getValue("payload"))) {
                    failures += "$id: this crate and the library read different claims"
                }
                if (!same(outcome.claims, case.getValue("expected"))) {
                    failures += "$id: the library does not read the document this was issued from"
                }
            }
        }
    }

    println("${cases.size} tokens; algorithms ${algs.joinToString(", ")}")
    println("\nfailures: ${failures.size}")
    failures.take(40).forEach { println("  $it") }
    if (failures.size > 40) println("  ... and ${failures.size - 40} more")
    if (failures.isNotEmpty()) exitProcess(1)
    println("every token agrees")
}
