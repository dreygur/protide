import type { MetaRecord } from "nextra";

export default {
  http: "HTTP",
  graphql: "GraphQL",
  websocket: "WebSocket",
  grpc: "gRPC",
  trpc: "tRPC",
  socketio: "Socket.IO",
} satisfies MetaRecord;
