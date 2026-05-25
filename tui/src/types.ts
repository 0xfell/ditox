export type Entry = {
  id: number;
  kind: "text" | "image";
  mime: string;
  content: string;
  preview: string;
  hash: string;
  favorite: boolean;
  created_at_ms: number;
  byte_len: number;
  source_app: string | null;
  blob_path: string | null;
  image_width: number | null;
  image_height: number | null;
};

export type EntryFilter = "all" | "text" | "images" | "favorites" | "today";

export type ListResponse = {
  entries: Entry[];
};

export type RpcSuccess<T> = {
  jsonrpc: "2.0";
  id: string;
  result: T;
};

export type RpcFailure = {
  jsonrpc: "2.0";
  id: string | null;
  error: {
    code: number;
    message: string;
    data?: unknown;
  };
};

export type RpcResponse<T> = RpcSuccess<T> | RpcFailure;
