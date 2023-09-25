import init, { Handler } from "./pkg/server.js";
import { serveDir } from "https://deno.land/std/http/file_server.ts";

await init();
const handler = await Handler.new();

Deno.serve((req) => {
  const u = new URL(req.url);
  if (u.pathname.startsWith("/pkg") || u.pathname.startsWith("/public")) {
    return serveDir(req);
  }
  return handler.serve(req);
});
