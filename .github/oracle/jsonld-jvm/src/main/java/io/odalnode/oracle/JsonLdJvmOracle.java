package io.odalnode.oracle;

import com.apicatalog.jsonld.Document;
import com.apicatalog.jsonld.JsonLd;
import com.apicatalog.jsonld.JsonLdException;
import com.apicatalog.jsonld.JsonLdException.ErrorCode;
import com.apicatalog.jsonld.JsonLd.Version;
import com.apicatalog.jsonld.Options;
import com.apicatalog.jsonld.Options.ProcessingPolicy;
import com.apicatalog.jsonld.loader.DocumentLoader;
import com.apicatalog.tree.io.TreeIO;
import com.apicatalog.tree.io.java.NativeAdapter;
import com.apicatalog.web.media.MediaType;
import jakarta.json.Json;
import jakarta.json.JsonArray;
import jakarta.json.JsonArrayBuilder;
import jakarta.json.JsonNumber;
import jakarta.json.JsonObject;
import jakarta.json.JsonObjectBuilder;
import jakarta.json.JsonReader;
import jakarta.json.JsonString;
import jakarta.json.JsonValue;

import java.io.IOException;
import java.io.InputStream;
import java.net.URI;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.ArrayList;
import java.util.Collection;
import java.util.HexFormat;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.TreeSet;
import java.util.regex.Pattern;

/** Judges the Rust-generated JSON-LD corpus with Titanium, without network access. */
public final class JsonLdJvmOracle {

    private static final Path ROOT = Path.of("").toAbsolutePath().normalize();
    private static final Path VENDORED_DIR = ROOT.resolve("crates/dpp-tests/fixtures/jsonld");
    private static final Map<String, JsonObject> VENDORED_CONTEXTS = new LinkedHashMap<>();
    private static final Map<String, VendoredContext> VENDORED = Map.of(
            "https://www.w3.org/ns/credentials/v2",
            new VendoredContext("credentials-v2.jsonld", "59955ced6697d61e03f2b2556febe5308ab16842846f5b586d7f1f7adec92734"),
            "https://www.w3.org/ns/did/v1",
            new VendoredContext("did-v1.jsonld", "4f3eae5568c9c5f036a082088f9e192019ee06faa78973c87ff91d5421b88dad"),
            "https://www.w3.org/ns/cid/v1",
            new VendoredContext("cid-v1.jsonld", "ea216ecc1cb02cd39b693dba2250141e270ba0bf95890be107dd9a9e8e43de85"));
    private static final Pattern ABSOLUTE_IRI = Pattern.compile("^[A-Za-z][A-Za-z0-9+.-]*:.*");
    private static final DocumentLoader LOADER = JsonLdJvmOracle::loadDocument;

    private JsonLdJvmOracle() {}

    public static void main(String[] args) {
        Path corpusPath = args.length == 0 ? ROOT.resolve("target/jsonld-oracle/corpus.json")
                : ROOT.resolve(args[0]).normalize();
        int failures = 0;
        int contextCount = 0;
        int documentCount = 0;
        try {
            verifyVendoredContexts();
            if (!Files.isRegularFile(corpusPath)) {
                throw new IllegalStateException("No corpus at " + corpusPath
                        + ". Build it with `EMIT_JSONLD_CORPUS=1 cargo test -p dpp-tests --test jsonld_oracle_corpus`.");
            }
            JsonObject corpus;
            try (InputStream input = Files.newInputStream(corpusPath);
                 JsonReader reader = Json.createReader(input)) {
                corpus = reader.readObject();
            }
            contextCount = corpus.getJsonArray("contexts").size();
            documentCount = corpus.getJsonArray("documents").size();

            System.out.println("Titanium JSON-LD 2.0.0-M2, processing mode json-ld-1.1");
            failures += controls();
            for (JsonValue entry : corpus.getJsonArray("contexts")) {
                JsonObject contextEntry = entry.asJsonObject();
                String label = contextEntry.getString("label");
                JsonValue context = contextEntry.get("context");
                List<String> problems = new ArrayList<>();
                JsonObject contextProbe = probe(context);
                try {
                    Collection<?> expanded = expand(contextProbe);
                    for (String path : lostRoots(contextProbe, expanded)) {
                        if (!path.equals("/@id")) {
                            problems.add(path + " expands to nothing");
                        }
                    }
                } catch (Exception error) {
                    problems.add(describe(error));
                }
                for (String base : List.of("https://www.w3.org/ns/credentials/v2", "https://www.w3.org/ns/did/v1")) {
                    if (containsContext(context, base)) {
                        continue;
                    }
                    try {
                        expand(layer(base, context));
                    } catch (Exception error) {
                        problems.add("cannot be layered after " + base + ": " + describe(error));
                    }
                }
                failures += report("context ", label, problems);
            }

            for (JsonValue entry : corpus.getJsonArray("documents")) {
                JsonObject documentEntry = entry.asJsonObject();
                String label = documentEntry.getString("label");
                JsonObject document = documentEntry.getJsonObject("document");
                List<String> problems = new ArrayList<>();
                try {
                    Collection<?> expanded = expand(document);
                    for (String path : lostRoots(document, expanded)) {
                        problems.add(path + " expands to nothing");
                    }
                    relativeIds(expanded, "", problems);
                } catch (Exception error) {
                    problems.add(describe(error));
                }
                failures += report("document", label, problems);
            }
        } catch (Exception error) {
            System.err.println("error: " + describe(error));
            System.exit(1);
        }

        if (failures != 0) {
            System.err.println("error: " + failures + " failure(s)");
            System.exit(1);
        }
        System.out.printf("All %d contexts and %d documents expanded with no undefined terms or relative identifiers.%n",
                contextCount, documentCount);
    }

    private static Collection<?> expand(JsonObject document) throws JsonLdException {
        return JsonLd.expand(nativeDocument(document), options());
    }

    private static Options options() {
        return Options.newOptions()
                .mode(Version.V1_1)
                .base((URI) null)
                .loader(LOADER)
                .undefinedTermsPolicy(ProcessingPolicy.Fail);
    }

    private static Map<String, ?> compact(Collection<?> expanded, JsonValue context) throws JsonLdException {
        TreeIO expandedTree = new TreeIO(expanded, NativeAdapter.instance());
        TreeIO contextTree = new TreeIO(nativeValue(normalizedContext(context)), NativeAdapter.instance());
        return JsonLd.compact(expandedTree, contextTree, options());
    }

    private static List<String> lostRoots(JsonObject original, Collection<?> expanded) throws JsonLdException {
        Map<String, Object> originalTree = nativeObject(original);
        Map<String, ?> compacted = compact(expanded, original.get("@context"));
        Set<String> lost = paths(originalTree);
        lost.removeAll(paths(nativeTree(compacted)));
        return lost.stream()
                .filter(path -> lost.stream().noneMatch(parent -> !parent.equals(path) && path.startsWith(parent + "/")))
                .sorted()
                .toList();
    }

    private static Set<String> paths(Object node) {
        Set<String> paths = new TreeSet<>();
        collectPaths(node, "", paths);
        return paths;
    }

    private static void collectPaths(Object node, String prefix, Set<String> paths) {
        if (node instanceof Map<?, ?> object) {
            object.forEach((key, value) -> {
                String name = key.toString();
                if (!name.equals("@context") && value != null) {
                    String path = prefix + "/" + name;
                    paths.add(path);
                    collectPaths(value, path, paths);
                }
            });
        } else if (node instanceof Collection<?> values) {
            values.forEach(value -> collectPaths(value, prefix, paths));
        }
    }

    private static Object nativeTree(Object node) {
        if (node instanceof TreeIO tree) {
            return nativeTree(NativeAdapter.adapt(tree.node(), tree.adapter()));
        }
        if (node instanceof Map<?, ?> object) {
            Map<String, Object> result = new LinkedHashMap<>();
            object.forEach((key, value) -> result.put(key.toString(), nativeTree(value)));
            return result;
        }
        if (node instanceof Collection<?> values) {
            return values.stream().map(JsonLdJvmOracle::nativeTree).toList();
        }
        return node;
    }

    private static Map<String, Object> nativeDocument(JsonObject document) {
        Map<String, Object> result = nativeObject(document);
        JsonValue context = document.get("@context");
        if (context != null) {
            result.put("@context", nativeValue(normalizedContext(context)));
        }
        return result;
    }

    private static JsonArray normalizedContext(JsonValue context) {
        // Titanium rejects identical protected terms repeated by layered contexts.
        // Inline only hash-verified fixtures and remove only exact duplicate definitions.
        JsonArrayBuilder normalized = Json.createArrayBuilder();
        Map<String, JsonValue> previousTerms = new LinkedHashMap<>();
        for (JsonValue entry : contextArray(context)) {
            JsonObject definitions = null;
            JsonValue normalizedEntry = entry;
            if (entry.getValueType() == JsonValue.ValueType.STRING) {
                String url = ((JsonString) entry).getString();
                definitions = VENDORED_CONTEXTS.get(url);
                if (definitions != null) {
                    JsonObjectBuilder filtered = Json.createObjectBuilder();
                    definitions.forEach((key, value) -> {
                        if (key.startsWith("@") || !value.equals(previousTerms.get(key))) {
                            filtered.add(key, value);
                        }
                    });
                    normalizedEntry = filtered.build();
                }
            } else if (entry.getValueType() == JsonValue.ValueType.OBJECT) {
                definitions = entry.asJsonObject();
            }
            normalized.add(normalizedEntry);
            if (definitions != null) {
                definitions.forEach((key, value) -> {
                    if (!key.startsWith("@")) {
                        previousTerms.put(key, value);
                    }
                });
            }
        }
        return normalized.build();
    }

    private static Document loadDocument(URI url, DocumentLoader.Options options) throws JsonLdException {
        VendoredContext context = VENDORED.get(url.toString());
        if (context == null) {
            throw new JsonLdException(ErrorCode.LOADING_DOCUMENT_FAILED,
                    "The oracle has no vendored copy of " + url + "; network loading is disabled.");
        }
        try (InputStream input = Files.newInputStream(VENDORED_DIR.resolve(context.file()))) {
            JsonObject json;
            try (JsonReader reader = Json.createReader(input)) {
                json = reader.readObject();
            }
            TreeIO document = new TreeIO(nativeObject(json), NativeAdapter.instance());
            return Document.of(document, MediaType.JSON_LD, url);
        } catch (IOException error) {
            throw new JsonLdException(ErrorCode.LOADING_DOCUMENT_FAILED, error);
        }
    }

    private static void verifyVendoredContexts() throws IOException, NoSuchAlgorithmException {
        MessageDigest sha256 = MessageDigest.getInstance("SHA-256");
        for (Map.Entry<String, VendoredContext> entry : VENDORED.entrySet()) {
            Path path = VENDORED_DIR.resolve(entry.getValue().file());
            if (!Files.isRegularFile(path)) {
                throw new IOException(entry.getKey() + ": " + path + " is missing");
            }
            String actual = HexFormat.of().formatHex(sha256.digest(Files.readAllBytes(path)));
            if (!actual.equals(entry.getValue().sha256())) {
                throw new IOException(entry.getKey() + ": " + entry.getValue().file()
                        + " does not match its recorded SHA-256");
            }
            try (InputStream input = Files.newInputStream(path);
                 JsonReader reader = Json.createReader(input)) {
                VENDORED_CONTEXTS.put(entry.getKey(), reader.readObject().getJsonObject("@context"));
            }
        }
    }

    private static Map<String, Object> nativeObject(JsonObject object) {
        Map<String, Object> result = new LinkedHashMap<>();
        object.forEach((key, value) -> result.put(key, nativeValue(value)));
        return result;
    }

    private static Object nativeValue(JsonValue value) {
        return switch (value.getValueType()) {
            case ARRAY -> value.asJsonArray().stream().map(JsonLdJvmOracle::nativeValue).toList();
            case OBJECT -> nativeObject(value.asJsonObject());
            case STRING -> ((JsonString) value).getString();
            case NUMBER -> ((JsonNumber) value).numberValue();
            case TRUE -> Boolean.TRUE;
            case FALSE -> Boolean.FALSE;
            case NULL -> null;
        };
    }

    private static JsonObject probe(JsonValue context) {
        JsonArray entries = contextArray(context);
        JsonObjectBuilder document = Json.createObjectBuilder()
                .add("@context", context)
                .add("@id", "urn:oracle:probe");
        for (JsonValue entry : entries) {
            if (entry.getValueType() == JsonValue.ValueType.OBJECT) {
                properties(entry.asJsonObject()).forEach(document::add);
            }
        }
        return document.build();
    }

    private static JsonObject properties(JsonObject definitions) {
        JsonObjectBuilder properties = Json.createObjectBuilder();
        definitions.forEach((term, definition) -> {
            if (term.startsWith("@")) {
                return;
            }
            switch (definition.getValueType()) {
                case NULL -> properties.add(term, "x");
                case STRING -> {
                    String iri = ((JsonString) definition).getString();
                    if (!iri.startsWith("@") && !iri.contains("://")) {
                        properties.add(term, "x");
                    }
                }
                case OBJECT -> {
                    JsonValue scoped = definition.asJsonObject().get("@context");
                    properties.add(term, scoped != null && scoped.getValueType() == JsonValue.ValueType.OBJECT
                            ? properties(scoped.asJsonObject()) : Json.createValue("x"));
                }
                default -> { }
            }
        });
        return properties.build();
    }

    private static JsonObject layer(String base, JsonValue context) {
        JsonArrayBuilder layered = Json.createArrayBuilder().add(base);
        contextArray(context).forEach(layered::add);
        return Json.createObjectBuilder()
                .add("@context", layered)
                .add("@id", "urn:oracle:probe")
                .addAll(propertiesForContext(context))
                .build();
    }

    private static JsonObjectBuilder propertiesForContext(JsonValue context) {
        JsonObjectBuilder properties = Json.createObjectBuilder();
        for (JsonValue entry : contextArray(context)) {
            if (entry.getValueType() == JsonValue.ValueType.OBJECT) {
                properties(entry.asJsonObject()).forEach(properties::add);
            }
        }
        return properties;
    }

    private static JsonArray contextArray(JsonValue context) {
        if (context.getValueType() == JsonValue.ValueType.ARRAY) {
            return context.asJsonArray();
        }
        return Json.createArrayBuilder().add(context).build();
    }

    private static boolean containsContext(JsonValue context, String url) {
        return contextArray(context).stream().anyMatch(value -> value.getValueType() == JsonValue.ValueType.STRING
                && ((JsonString) value).getString().equals(url));
    }

    private static int controls() {
        int failures = 0;
        JsonObject redefined = Json.createObjectBuilder()
                .add("@context", Json.createArrayBuilder()
                        .add("https://www.w3.org/ns/credentials/v2")
                        .add(Json.createObjectBuilder().add("name", "https://example.org/another-name")))
                .add("name", "x")
                .build();
        failures += expectRejected("protected term redefinition", redefined, ErrorCode.PROTECTED_TERM_REDEFINITION);

        JsonObject undefined = Json.createObjectBuilder()
                .add("@context", Json.createObjectBuilder().add("dpp", "https://example.org/dpp#"))
                .add("undefinedKey", "x")
                .build();
        failures += expectRejected("undefined property", undefined, ErrorCode.UNDEFINED_TERM);

        JsonObject remote = Json.createObjectBuilder()
                .add("@context", "https://example.org/never-vendored")
                .add("a", "x")
                .build();
        failures += expectRejected("unvendored context", remote, ErrorCode.LOADING_REMOTE_CONTEXT_FAILED);

        JsonObject relative = Json.createObjectBuilder()
                .add("@context", Json.createObjectBuilder().add("dpp", "https://example.org/dpp#"))
                .add("@id", "a-bare-uuid")
                .add("dpp:x", "y")
                .build();
        try {
            List<String> identifiers = new ArrayList<>();
            relativeIds(expand(relative), "", identifiers);
            if (identifiers.stream().noneMatch(value -> value.contains("a-bare-uuid"))) {
                System.err.println("FAIL  control relative identifier was not reported");
                failures++;
            }
        } catch (Exception error) {
            System.err.println("FAIL  control relative identifier: " + describe(error));
            failures++;
        }
        if (failures == 0) {
            System.out.println("controls: protected redefinition, undefined term and unvendored context rejected; relative identifier detected\n");
        }
        return failures;
    }

    private static int expectRejected(String label, JsonObject document, ErrorCode expectedCode) {
        try {
            expand(document);
            System.err.println("FAIL  control " + label + " was accepted");
            return 1;
        } catch (JsonLdException error) {
            if (error.code() != expectedCode) {
                System.err.println("FAIL  control " + label + " failed with " + describe(error)
                        + ", expected " + expectedCode);
                return 1;
            }
            System.out.println("ok    control " + label + " rejected");
            return 0;
        }
    }

    private static void relativeIds(Object node, String path, List<String> problems) {
        if (node instanceof Map<?, ?> object) {
            Object id = object.get("@id");
            if (id instanceof String identifier) {
                if (!identifier.startsWith("_:") && !ABSOLUTE_IRI.matcher(identifier).matches()) {
                    problems.add((path.isEmpty() ? "/" : path) + " is named " + identifier + ", a relative IRI");
                }
            }
            object.forEach((key, value) -> {
                if (!key.equals("@id") && !key.equals("@value")) {
                    relativeIds(value, path + "/" + key, problems);
                }
            });
        } else if (node instanceof Collection<?> values) {
            values.forEach(value -> relativeIds(value, path, problems));
        }
    }

    private static int report(String kind, String label, List<String> problems) {
        if (problems.isEmpty()) {
            System.out.println("ok    " + kind + " " + label);
            return 0;
        }
        System.err.println("FAIL  " + kind + " " + label);
        problems.forEach(problem -> System.err.println("        " + problem));
        return 1;
    }

    private static String describe(Exception error) {
        if (error instanceof JsonLdException jsonLdError) {
            Throwable cause = jsonLdError.getCause();
            return jsonLdError.code().name() + ": " + jsonLdError.getMessage()
                    + (cause == null ? "" : " (cause: " + cause.getClass().getSimpleName() + ": " + cause.getMessage() + ")");
        }
        return error.getClass().getSimpleName() + ": " + error.getMessage();
    }

    private record VendoredContext(String file, String sha256) {}
}