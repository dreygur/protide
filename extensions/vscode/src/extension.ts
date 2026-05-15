import * as path from "path";
import * as os from "os";
import { workspace, ExtensionContext } from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient;

export function activate(context: ExtensionContext) {
  const serverBin = resolveServerBin(context);

  const serverOptions: ServerOptions = {
    run: { command: serverBin, transport: TransportKind.stdio },
    debug: { command: serverBin, transport: TransportKind.stdio },
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: "file", language: "http" },
      { scheme: "file", pattern: "**/*.http" },
      { scheme: "file", pattern: "**/*.rest" },
    ],
    synchronize: {
      fileEvents: workspace.createFileSystemWatcher("**/*.http"),
    },
  };

  client = new LanguageClient(
    "protide-lsp",
    "Protide Language Server",
    serverOptions,
    clientOptions
  );

  client.start();
}

export function deactivate(): Thenable<void> | undefined {
  return client?.stop();
}

function resolveServerBin(context: ExtensionContext): string {
  // Check bundled binary first
  const platform = os.platform();
  const ext = platform === "win32" ? ".exe" : "";
  const bundled = context.asAbsolutePath(
    path.join("bin", `protide-lsp${ext}`)
  );

  // Fall back to PATH
  return process.env.PROTIDE_LSP_BIN ?? bundled;
}
