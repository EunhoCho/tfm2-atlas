const net = require("node:net");

const MAX_FRAME_BYTES = 16 * 1024 * 1024;

function encodeFrame(value) {
  const payload = Buffer.from(JSON.stringify(value), "utf8");
  if (payload.length > MAX_FRAME_BYTES) throw new Error("frame_too_large");
  const frame = Buffer.allocUnsafe(payload.length + 4);
  frame.writeUInt32BE(payload.length, 0);
  payload.copy(frame, 4);
  return frame;
}

class FrameDecoder {
  constructor() {
    this.buffer = Buffer.alloc(0);
  }

  push(chunk) {
    this.buffer = Buffer.concat([this.buffer, Buffer.from(chunk)]);
    const values = [];
    while (this.buffer.length >= 4) {
      const length = this.buffer.readUInt32BE(0);
      if (length > MAX_FRAME_BYTES) throw new Error("frame_too_large");
      if (this.buffer.length < length + 4) break;
      values.push(JSON.parse(this.buffer.subarray(4, length + 4).toString("utf8")));
      this.buffer = this.buffer.subarray(length + 4);
    }
    return values;
  }
}

class BridgeClient {
  constructor({ host = "127.0.0.1", port = 28452, timeoutMs = 8000 } = {}) {
    this.host = host;
    this.port = port;
    this.timeoutMs = timeoutMs;
    this.nextRequestId = 1;
  }

  request(command, payload = {}) {
    const requestId = this.nextRequestId++;
    const request = { request_id: requestId, command, payload };
    return new Promise((resolve, reject) => {
      const decoder = new FrameDecoder();
      const socket = net.createConnection({ host: this.host, port: this.port });
      const timeout = setTimeout(() => {
        socket.destroy();
        reject(Object.assign(new Error(`bridge_timeout:${command}`), { code: "bridge_timeout" }));
      }, this.timeoutMs);
      const finish = (callback) => {
        clearTimeout(timeout);
        socket.destroy();
        callback();
      };
      socket.once("error", (error) => finish(() => reject(error)));
      socket.on("data", (chunk) => {
        let messages;
        try {
          messages = decoder.push(chunk);
        } catch (error) {
          finish(() => reject(error));
          return;
        }
        const response = messages[0];
        if (!response) return;
        if (response.request_id !== requestId) {
          finish(() => reject(new Error("request_id_mismatch")));
        } else if (!response.ok) {
          const error = new Error(response.error?.message || "bridge_error");
          error.code = response.error?.code || "bridge_error";
          finish(() => reject(error));
        } else {
          finish(() => resolve(response.data));
        }
      });
      socket.once("connect", () => socket.write(encodeFrame(request)));
    });
  }
}

class StateEventClient {
  constructor({ host = "127.0.0.1", port = 28452, reconnectMs = 1000 } = {}) {
    this.host = host;
    this.port = port;
    this.reconnectMs = reconnectMs;
    this.socket = null;
    this.stopped = true;
    this.onEvent = null;
  }

  start(onEvent) {
    this.onEvent = onEvent;
    this.stopped = false;
    this.connect();
  }

  connect() {
    if (this.stopped || this.socket) return;
    const decoder = new FrameDecoder();
    const socket = net.createConnection({ host: this.host, port: this.port });
    this.socket = socket;
    socket.once("connect", () => socket.write(encodeFrame({ request_id: 1, command: "SUBSCRIBE_STATE", payload: {} })));
    socket.on("data", (chunk) => {
      try {
        for (const message of decoder.push(chunk)) {
          if (message?.event === "STATE_CHANGED") this.onEvent?.(message);
          else if (message?.ok && message?.data?.subscribed === true) {
            this.onEvent?.({
              event: "STATE_CHANGED",
              scopes: ["EDITOR_CHANGED"],
              changed_at_unix_ms: Date.now(),
            });
          }
        }
      } catch {
        socket.destroy();
      }
    });
    const reconnect = () => {
      if (this.socket !== socket) return;
      this.socket = null;
      if (!this.stopped) setTimeout(() => this.connect(), this.reconnectMs);
    };
    socket.once("error", reconnect);
    socket.once("close", reconnect);
  }

  stop() {
    this.stopped = true;
    this.socket?.destroy();
    this.socket = null;
  }
}

module.exports = { BridgeClient, FrameDecoder, MAX_FRAME_BYTES, StateEventClient, encodeFrame };
