// CSS variable references for method and protocol colors.
// All hex values live in tokens.css - this file just names the variables.
// Mapping matches theme.rs method_color() exactly.
export const METHOD_COLORS: Record<string, string> = {
  GET:     "var(--color-method-get)",
  POST:    "var(--color-method-post)",
  PUT:     "var(--color-method-put)",
  PATCH:   "var(--color-method-patch)",
  DELETE:  "var(--color-method-delete)",
  HEAD:    "var(--color-method-head)",
  OPTIONS: "var(--color-method-options)",
};

export const PROTOCOL_COLORS: Record<string, string> = {
  WebSocket:  "var(--color-proto-ws)",
  gRPC:       "var(--color-proto-grpc)",
  GraphQL:    "var(--color-proto-graphql)",
  tRPC:       "var(--color-proto-trpc)",
  "Socket.IO":"var(--color-proto-sio)",
};
