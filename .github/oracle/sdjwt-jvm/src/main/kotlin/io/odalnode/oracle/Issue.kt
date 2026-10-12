package io.odalnode.oracle

import com.nimbusds.jose.JWSAlgorithm
import com.nimbusds.jose.JWSHeader
import com.nimbusds.jose.JWSSigner
import com.nimbusds.jose.jca.JCAContext
import com.nimbusds.jose.jwk.Curve
import com.nimbusds.jose.jwk.OctetKeyPair
import com.nimbusds.jose.util.Base64URL
import eu.europa.ec.eudi.sdjwt.NimbusSdJwtOps
import eu.europa.ec.eudi.sdjwt.cnf
import eu.europa.ec.eudi.sdjwt.dsl.values.sdJwt
import eu.europa.ec.eudi.sdjwt.vc.ClaimPath
import kotlinx.coroutines.runBlocking
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import java.io.File
import java.security.KeyPairGenerator
import java.security.PrivateKey
import java.security.Signature
import java.security.spec.NamedParameterSpec
import java.util.Base64
import java.util.Date

/**
 * The other direction: tokens this library issues, for this crate to read.
 *
 * The library signs through a Nimbus `JWSSigner`, so Ed25519 is signed here with the
 * JDK's own provider and needs no extra library. For each token the library
 * verifies it first, and what it says the claims are is the `expected` value. The
 * file has the shape `.github/oracle/sdjwt/issue.mjs` writes, so
 * `sd_jwt::oracle_tests::reverse` reads it with `CHECK_SDJWT_ORACLE_ISSUED`.
 *
 * Not reproducible byte for byte: the library draws its salts from the system
 * generator, and a fresh key is made each run. The shapes are fixed.
 */
class JdkEd25519Signer(private val key: PrivateKey) : JWSSigner {
    private val context = JCAContext()

    override fun supportedJWSAlgorithms(): Set<JWSAlgorithm> = setOf(JWSAlgorithm.EdDSA)

    override fun getJCAContext(): JCAContext = context

    override fun sign(header: JWSHeader, signingInput: ByteArray): Base64URL {
        val signer = Signature.getInstance("Ed25519")
        signer.initSign(key)
        signer.update(signingInput)
        return Base64URL.encode(signer.sign())
    }
}

private val props = (0..5).map { "n$it" }

private class Case(val id: String, val token: String, val challenge: Challenge? = null)

fun main(args: Array<String>) {
    val out = args.firstOrNull() ?: error("usage: IssueKt <issued-kotlin.json>")
    val pair = KeyPairGenerator.getInstance("Ed25519").apply { initialize(NamedParameterSpec.ED25519) }.generateKeyPair()
    // The last 32 bytes of the SubjectPublicKeyInfo are the raw key (RFC 8410 section 4).
    val raw = pair.public.encoded.takeLast(32).toByteArray()
    val verifier = JdkEd25519Verifier(raw)
    val cases = mutableListOf<Case>()
    val problems = mutableListOf<String>()

    fun record(id: String, token: String, challenge: Challenge? = null) {
        cases += Case(id, token, challenge)
    }

    // A Holder, for presentations bound to its key: the Issuer names it in `cnf`,
    // and the library signs the Key Binding JWT with it through the JDK.
    val holderPair = KeyPairGenerator.getInstance("Ed25519").apply { initialize(NamedParameterSpec.ED25519) }.generateKeyPair()
    val holderJwk = OctetKeyPair.Builder(Curve.Ed25519, Base64URL.encode(holderPair.public.encoded.takeLast(32).toByteArray())).build()
    // Now, because Nimbus checks a Key Binding JWT's time claims against its own clock.
    val now = System.currentTimeMillis() / 1000

    runBlocking {
        with(NimbusSdJwtOps) {
            val issuer = issuer(signer = JdkEd25519Signer(pair.private), signAlgorithm = JWSAlgorithm.EdDSA)

            // Every hidden/shown combination of six properties, issued and with only the
            // lowest hidden property presented.
            for (mask in 0 until (1 shl props.size)) {
                val spec = sdJwt {
                    claim("iss", "https://issuer.example")
                    props.forEachIndexed { i, name ->
                        if (mask and (1 shl i) != 0) sdClaim(name, JsonPrimitive("v$i")) else claim(name, JsonPrimitive("v$i"))
                    }
                }
                val sdJwt = issuer.issue(spec).getOrThrow()
                record("flat/$mask/issued", sdJwt.serialize())
                val first = props.indexOfFirst { mask and (1 shl props.indexOf(it)) != 0 }
                if (first >= 0) {
                    sdJwt.present(setOf(ClaimPath.claim(props[first])))?.let { record("flat/$mask/first", it.serialize()) }
                }
            }

            // Arrays up to four long: every combination of hidden elements.
            for (length in 1..4) {
                for (mask in 0 until (1 shl length)) {
                    val spec = sdJwt {
                        claim("iss", "https://issuer.example")
                        arrClaim("list") {
                            for (i in 0 until length) {
                                if (mask and (1 shl i) != 0) sdClaim(JsonPrimitive("e$i")) else claim(JsonPrimitive("e$i"))
                            }
                        }
                    }
                    val sdJwt = issuer.issue(spec).getOrThrow()
                    record("array/$length/$mask/issued", sdJwt.serialize())
                    val first = (0 until length).firstOrNull { mask and (1 shl it) != 0 }
                    if (first != null) {
                        sdJwt.present(setOf(ClaimPath.claim("list").arrayElement(first)))?.let {
                            record("array/$length/$mask/first", it.serialize())
                        }
                    }
                }
            }

            // Nested, recursive, and array-of-objects shapes.
            val shapes = mapOf(
                "nested/inner-hidden" to sdJwt {
                    claim("iss", "https://issuer.example")
                    objClaim("address") { sdClaim("city", JsonPrimitive("Berlin")); claim("zip", JsonPrimitive("10115")) }
                },
                "nested/object-hidden" to sdJwt {
                    claim("iss", "https://issuer.example")
                    sdObjClaim("address") { claim("city", JsonPrimitive("Berlin")); claim("zip", JsonPrimitive("10115")) }
                },
                "recursive/object-and-members-hidden" to sdJwt {
                    claim("iss", "https://issuer.example")
                    sdObjClaim("address") { sdClaim("city", JsonPrimitive("Berlin")); sdClaim("zip", JsonPrimitive("10115")) }
                },
                "recursive/three-deep" to sdJwt {
                    claim("iss", "https://issuer.example")
                    sdObjClaim("a") { sdObjClaim("b") { sdClaim("c", JsonPrimitive("deep")) } }
                },
                "array/of-objects" to sdJwt {
                    claim("iss", "https://issuer.example")
                    arrClaim("people") {
                        sdObjClaim { sdClaim("name", JsonPrimitive("A")); claim("age", JsonPrimitive(1)) }
                        objClaim { claim("name", JsonPrimitive("B")); sdClaim("age", JsonPrimitive(2)) }
                    }
                },
                "array/hidden-array" to sdJwt {
                    claim("iss", "https://issuer.example")
                    sdArrClaim("tags") { sdClaim(JsonPrimitive("x")); claim(JsonPrimitive("y")) }
                },
                "array/in-array" to sdJwt {
                    claim("iss", "https://issuer.example")
                    arrClaim("m") { arrClaim { sdClaim(JsonPrimitive(1)); claim(JsonPrimitive(2)) }; sdArrClaim { claim(JsonPrimitive(3)) } }
                },
                "mixed/types" to sdJwt {
                    claim("iss", "https://issuer.example")
                    sdClaim("b", JsonPrimitive(true)); sdClaim("n", JsonPrimitive(42)); sdClaim("s", JsonPrimitive("é\"\\ ☃"))
                    sdClaim("list", JsonArray(listOf(JsonPrimitive(1), JsonPrimitive("two"))))
                    sdClaim("obj", JsonObject(mapOf("k" to JsonPrimitive("v"))))
                },
                "decoys/root" to sdJwt(minimumDigests = 8) {
                    claim("iss", "https://issuer.example")
                    sdClaim("name", JsonPrimitive("Alice")); claim("age", JsonPrimitive(30))
                },
                "decoys/nested" to sdJwt(minimumDigests = 6) {
                    claim("iss", "https://issuer.example")
                    objClaim("address", minimumDigests = 9) { sdClaim("city", JsonPrimitive("Berlin")) }
                },
                "decoys/array" to sdJwt {
                    claim("iss", "https://issuer.example")
                    arrClaim("list", minimumDigests = 6) { sdClaim(JsonPrimitive("a")); claim(JsonPrimitive("b")); sdClaim(JsonPrimitive("c")) }
                },
                "decoys/everything-hidden" to sdJwt(minimumDigests = 10) {
                    sdClaim("a", JsonPrimitive(1)); sdClaim("b", JsonPrimitive(2))
                },
            )
            // Bound to the Holder: the library issues with `cnf`, presents part, and
            // adds a Key Binding JWT for one Verifier's challenge.
            val kbIssuer = { nonce: String ->
                kbJwtIssuer(JdkEd25519Signer(holderPair.private), JWSAlgorithm.EdDSA, holderJwk) {
                    audience("https://verifier.example")
                    claim("nonce", nonce)
                    issueTime(Date(now * 1000))
                }
            }
            val boundShapes = listOf(
                Triple("key-binding/some-shown", sdJwt {
                    claim("iss", "https://issuer.example"); cnf(holderJwk)
                    sdClaim("given_name", JsonPrimitive("Erika")); sdClaim("family_name", JsonPrimitive("Mustermann"))
                }, setOf(ClaimPath.claim("given_name"))),
                Triple("key-binding/nothing-shown", sdJwt {
                    claim("iss", "https://issuer.example"); cnf(holderJwk)
                    sdClaim("given_name", JsonPrimitive("Erika"))
                }, setOf(ClaimPath.claim("iss"))),
                Triple("key-binding/array-element", sdJwt {
                    claim("iss", "https://issuer.example"); cnf(holderJwk)
                    arrClaim("list") { sdClaim(JsonPrimitive("a")); claim(JsonPrimitive("b")); sdClaim(JsonPrimitive("c")) }
                }, setOf(ClaimPath.claim("list").arrayElement(2))),
                Triple("key-binding/recursive", sdJwt {
                    claim("iss", "https://issuer.example"); cnf(holderJwk)
                    sdObjClaim("address") { sdClaim("city", JsonPrimitive("Berlin")); sdClaim("zip", JsonPrimitive("10115")) }
                }, setOf(ClaimPath.claim("address").claim("city"))),
            )
            for ((id, spec, query) in boundShapes) {
                val nonce = "nonce-$id"
                runCatching {
                    val presented = checkNotNull(issuer.issue(spec).getOrThrow().present(query)) { "nothing to present" }
                    presented.serializeWithKeyBinding(kbIssuer(nonce)).getOrThrow()
                }.fold(
                    onSuccess = { record("$id/presented", it, Challenge(nonce, "https://verifier.example", now, 0)) },
                    onFailure = { problems += "$id: ${it::class.simpleName}: ${it.message}" },
                )
            }

            for ((id, spec) in shapes) {
                runCatching { issuer.issue(spec).getOrThrow() }.fold(
                    onSuccess = { record("$id/issued", it.serialize()) },
                    onFailure = { problems += "$id: ${it::class.simpleName}: ${it.message}" },
                )
            }
        }
    }

    // What the library itself says each token means.
    val written = cases.mapNotNull { case ->
        val verdict = case.challenge?.let { judgeBound(case.token, verifier, it) } ?: judge(case.token, verifier)
        when (verdict) {
            is Outcome.Accepted -> JsonObject(
                buildMap {
                    put("id", JsonPrimitive(case.id))
                    put("verdict", JsonPrimitive("accept"))
                    put("token", JsonPrimitive(case.token))
                    put("expected", verdict.claims)
                    case.challenge?.let { ch ->
                        put("keyBinding", JsonPrimitive(true))
                        put(
                            "challenge",
                            JsonObject(
                                mapOf(
                                    "nonce" to JsonPrimitive(ch.nonce),
                                    "aud" to JsonPrimitive(ch.aud),
                                    "now" to JsonPrimitive(ch.now),
                                    "window" to JsonPrimitive(ch.window),
                                ),
                            ),
                        )
                    }
                },
            )
            is Outcome.Refused -> {
                problems += "${case.id}: the library cannot read its own token (${verdict.why})"
                null
            }
        }
    }

    if (problems.isNotEmpty()) {
        System.err.println("${problems.size} problem(s):")
        problems.forEach { System.err.println("  $it") }
        kotlin.system.exitProcess(1)
    }
    File(out).also { it.parentFile?.mkdirs() }.writeText(
        JsonObject(
            mapOf(
                "publicKey" to JsonPrimitive(Base64.getUrlEncoder().withoutPadding().encodeToString(raw)),
                "cases" to JsonArray(written),
            ),
        ).toString(),
    )
    println("eudi-lib-jvm-sdjwt-kt issued ${written.size} tokens to $out")
}
