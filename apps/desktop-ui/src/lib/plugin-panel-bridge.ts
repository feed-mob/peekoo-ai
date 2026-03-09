const BRIDGE_REQUEST_TYPE = "peekoo-plugin-invoke";
const BRIDGE_RESPONSE_TYPE = "peekoo-plugin-invoke-result";

const BRIDGE_SCRIPT = `<script>
(() => {
  const requestType = "${BRIDGE_REQUEST_TYPE}";
  const responseType = "${BRIDGE_RESPONSE_TYPE}";
  const pending = new Map();

  window.addEventListener("message", (event) => {
    const data = event.data;
    if (!data || data.type !== responseType || !data.id) {
      return;
    }

    const handlers = pending.get(data.id);
    if (!handlers) {
      return;
    }

    pending.delete(data.id);

    if (data.ok) {
      handlers.resolve(data.result);
      return;
    }

    handlers.reject(data.error ?? "Plugin invoke failed");
  });

  window.__TAURI__ = window.__TAURI__ || {};
  window.__TAURI__.core = window.__TAURI__.core || {};
  window.__TAURI__.core.invoke = (command, payload = {}) => {
    const id = crypto.randomUUID();

    return new Promise((resolve, reject) => {
      pending.set(id, { resolve, reject });
      window.parent.postMessage({
        type: requestType,
        id,
        command,
        payload,
      }, "*");
    });
  };
})();
</script>`;

export function injectPluginPanelBridge(html: string): string {
  if (html.includes(BRIDGE_REQUEST_TYPE)) {
    return html;
  }

  if (html.includes("</head>")) {
    return html.replace("</head>", `${BRIDGE_SCRIPT}</head>`);
  }

  return `${BRIDGE_SCRIPT}${html}`;
}

export { BRIDGE_REQUEST_TYPE, BRIDGE_RESPONSE_TYPE };
