const invoke = window.__TAURI__?.core?.invoke;
const list = document.getElementById("running");
const msg = document.getElementById("msg");

function setMessage(text, bad = false) {
  msg.textContent = text;
  msg.className = bad ? "msg bad" : "msg";
}

// Split on whitespace but honour quotes, so a datadir with a space works.
function tokenize(input) {
  const out = [];
  let cur = "";
  let quote = null;
  for (const ch of input) {
    if (quote) {
      if (ch === quote) quote = null;
      else cur += ch;
    } else if (ch === '"' || ch === "'") {
      quote = ch;
    } else if (/\s/.test(ch)) {
      if (cur) {
        out.push(cur);
        cur = "";
      }
    } else {
      cur += ch;
    }
  }
  if (cur) out.push(cur);
  return out;
}

async function refresh() {
  if (!invoke) return setMessage("Tauri bridge unavailable — run via `cargo tauri dev`.", true);
  const rows = await invoke("running_services");
  list.innerHTML = "";
  if (!rows.length) {
    list.innerHTML = '<li class="empty">Nothing running.</li>';
    return;
  }
  for (const row of rows) {
    const li = document.createElement("li");
    li.textContent = `${row.name} · pid ${row.pid} `;
    const stop = document.createElement("button");
    stop.type = "button";
    stop.textContent = "Stop";
    stop.addEventListener("click", async () => {
      try {
        await invoke("stop_service", { pid: row.pid });
        setMessage(`stopped ${row.name}`);
      } catch (err) {
        setMessage(String(err), true);
      }
      refresh();
    });
    li.appendChild(stop);
    list.appendChild(li);
  }
}

const DATA = "/Users/cyasitha/Library/Application Support/Stackdeck";
const PRESETS = {
  mariadb: {
    name: "mariadb",
    program: "/opt/homebrew/opt/mariadb/bin/mariadbd",
    args: `--datadir="${DATA}/data" --port=3306 --socket=/tmp/stackdeck.sock`,
  },
  "php-8.5": {
    name: "php8.5",
    program: "/opt/homebrew/opt/php/sbin/php-fpm",
    args: "--fpm-config=/Users/cyasitha/Documents/Projects/rust/stackdeck/presets/php85-fpm.conf",
  },
  "php-8.3": {
    name: "php8.3",
    program: "/opt/homebrew/opt/php@8.3/sbin/php-fpm",
    args: "--fpm-config=/Users/cyasitha/Documents/Projects/rust/stackdeck/presets/php83-fpm.conf",
  },
  "php-8.1": {
    name: "php8.1",
    program: "/opt/homebrew/opt/php@8.1/sbin/php-fpm",
    args: "--fpm-config=/Users/cyasitha/Documents/Projects/rust/stackdeck/presets/php81-fpm.conf",
  },
};

document.getElementById("preset").addEventListener("change", (event) => {
  const preset = PRESETS[event.target.value];
  if (!preset) return;
  const form = document.getElementById("start-form");
  form.elements.name.value = preset.name;
  form.elements.program.value = preset.program;
  form.elements.args.value = preset.args;
});

document.getElementById("start-form").addEventListener("submit", async (event) => {
  event.preventDefault();
  const data = new FormData(event.target);
  const args = tokenize(String(data.get("args") ?? ""));
  try {
    const pid = await invoke("start_service", {
      name: String(data.get("name")),
      program: String(data.get("program")),
      args,
      cwd: null,
    });
    setMessage(`started ${data.get("name")} (pid ${pid})`);
  } catch (err) {
    setMessage(String(err), true);
  }
  refresh();
});

document.getElementById("refresh").addEventListener("click", refresh);
refresh();
