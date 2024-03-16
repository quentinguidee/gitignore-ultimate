import { join } from "path";
import { ExtensionContext } from "vscode";
import {
    LanguageClient,
    ServerOptions,
    LanguageClientOptions as ClientOptions,
} from "vscode-languageclient/node";

let client: LanguageClient;

const BIN_NAME = "server";

function serverOptions(ctx: ExtensionContext): ServerOptions {
    let ext = "";
    if (process.platform === "win32") {
        ext = ".exe";
    }
    return {
        command: ctx.asAbsolutePath(join("bin", BIN_NAME + ext)),
    };
}

function clientOptions(): ClientOptions {
    return {
        documentSelector: [{ scheme: "file", language: "ignore" }],
    };
}

export function startClient(ctx: ExtensionContext) {
    client = new LanguageClient(
        "gitignore-ultimate",
        "Gitignore Ultimate",
        serverOptions(ctx),
        clientOptions(),
    );
    client.start();
}

export function stopClient() {
    client?.stop();
}
