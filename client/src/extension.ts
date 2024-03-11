import * as vscode from "vscode";
import { startClient, stopClient } from "./client";

export function activate(context: vscode.ExtensionContext) {
    console.log("gitignore ultimate activated!");
    startClient(context);
}

export function deactivate() {
    stopClient();
}
