const { LanguageClient } = require("vscode-languageclient");

module.exports = {
    activate(context) {
        console.log('hello from lsp-gpt client for vscode!');
        const command = "lsp-gpt";

        const serverOptions = {
            run: { command: command },
            debug: { command: command },
        };
        const clientOptions = {
            documentSelector: [
                {
                    scheme: "file",
                    language: "latex"
                }
            ],
        };
        client = new LanguageClient(
            "lsp-gpt-prototype",
            "lsp-gpt prototype",
            serverOptions,
            clientOptions,
        );
        context.subscriptions.push(client.start())
    },
}
