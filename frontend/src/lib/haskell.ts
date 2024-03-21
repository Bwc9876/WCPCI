import type { Monaco } from "@monaco-editor/loader";

const tokenStuff = {
    tokenizer: {
        root: [
            ["--.*$", "comment"], // Single-line comments
            ["{-", "comment", "@comment"], // Multi-line comments start
            ["-}", "comment", "@pop"], // Multi-line comments end
            [
                "\\b(import|module|data|type|class|instance|where|do|let|case|of|if|then|else|in)\\b",
                "keyword"
            ], // Keywords
            ["\\b(True|False)\\b", "boolean"], // Boolean literals
            ["\\b([A-Z][\\w']*)\\b", "type.identifier"], // Type names
            ["\\b([a-z][\\w']*)\\b", "identifier"], // Variable names
            ["\\b[0-9]+(.[0-9]+)?\\b", "number"], // Numbers
            ['"', "string", "@string"] // Strings start
        ],
        comment: [
            ["[^\\-}\\{]+", "comment"],
            ["{-", "comment", "@comment"],
            ["-}", "comment", "@pop"],
            ["[\\-}]|\\{(?=\\s*\\w)", "comment"]
        ],
        string: [
            ['[^\\\\"]+', "string"],
            ["\\\\.", "string.escape"],
            ['"', "string", "@pop"]
        ]
    }
};

export default (monaco: Monaco) => {
    monaco.languages.register({
        extensions: ["haskell"],
        id: "haskell"
    });
    monaco.languages.setLanguageConfiguration("haskell", {
        indentationRules: {
            decreaseIndentPattern: /\]/,
            increaseIndentPattern: /\[/,
            indentNextLinePattern: null,
            unIndentedLinePattern: null
        }
    });
    monaco.languages.setMonarchTokensProvider("haskell", tokenStuff as any);
};
