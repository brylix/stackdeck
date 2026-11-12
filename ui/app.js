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

const setupList = document.getElementById("setup");
let pollTimer = null;

function pollWhileSetup() {
  clearInterval(pollTimer);
  pollTimer = setInterval(renderSetup, 3000);
}

async function renderSetup() {
  if (!invoke) return;
  let state;
  try {
    state = await invoke("setup_state");
  } catch (err) {
    setupList.innerHTML = `<li class="empty">${err}</li>`;
    return;
  }

  if (!state.brew) {
    setupList.innerHTML = '<li class="empty">Homebrew not found at /opt/homebrew/bin/brew.</li>';
    clearInterval(pollTimer);
    return;
  }

  const rows = [
    ["mariadb", "MariaDB", state.mariadb_installed, state.mariadb_provisioned],
    ["php", "PHP (latest)", state.php_installed, true],
    ["phpmyadmin", "phpMyAdmin", state.phpmyadmin_installed, state.phpmyadmin_provisioned],
  ];

  setupList.innerHTML = "";
  let pending = false;
  for (const [id, label, installed, provisioned] of rows) {
    const li = document.createElement("li");
    li.textContent = label + " ";

    const status = document.createElement("span");
    status.className = "status";
    const button = document.createElement("button");
    button.type = "button";

    if (!installed) {
      status.textContent = "not installed";
      status.classList.add("warn");
      button.textContent = "Install";
      pending = true;
      button.addEventListener("click", async () => {
        button.disabled = true;
        button.textContent = "Installing…";
        await invoke("install_component", { component: id });
        pollWhileSetup();
      });
    } else if (!provisioned) {
      status.textContent = "installed, not set up";
      status.classList.add("warn");
      button.textContent = "Set up";
      button.addEventListener("click", async () => {
        button.disabled = true;
        try {
          await invoke("provision_component", { component: id });
          setMessage(`${label} configured`);
        } catch (err) {
          setMessage(String(err), true);
        }
        renderSetup();
      });
    } else {
      status.textContent = "ready";
      status.classList.add("ok");
      button.remove();
    }

    li.appendChild(status);
    if (button.isConnected !== false && button.textContent) li.appendChild(button);
    setupList.appendChild(li);
  }

  if (pending) pollWhileSetup();
  else clearInterval(pollTimer);
}

async function renderDetected() {
  if (!invoke) return;
  const list = document.getElementById("detected");
  let rows;
  try {
    rows = await invoke("discover_services");
  } catch {
    return;
  }
  list.innerHTML = "";
  if (!rows.length) {
    list.innerHTML = '<li class="empty">Nothing detected.</li>';
    return;
  }
  for (const row of rows) {
    const li = document.createElement("li");
    li.textContent = `${row.name} · pid ${row.pid} `;

    const badge = document.createElement("span");
    badge.className = "status warn";
    badge.textContent = "unmanaged";

    const track = document.createElement("button");
    track.type = "button";
    track.textContent = "Track";
    track.addEventListener("click", async () => {
      try {
        await invoke("adopt_service", { pid: row.pid, name: row.name });
        setMessage(`now tracking ${row.name}`);
      } catch (err) {
        setMessage(String(err), true);
      }
      renderDetected();
      refresh();
    });

    const stop = document.createElement("button");
    stop.type = "button";
    stop.textContent = "Stop";
    stop.addEventListener("click", async () => {
      try {
        await invoke("stop_service", { pid: row.pid });
      } catch (err) {
        setMessage(String(err), true);
      }
      renderDetected();
    });

    li.appendChild(badge);
    li.appendChild(track);
    li.appendChild(stop);
    list.appendChild(li);
  }
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
  phpmyadmin: {
    name: "phpmyadmin",
    program: "/opt/homebrew/opt/php@8.3/bin/php",
    args: `-S 127.0.0.1:8081 -t "${DATA}/phpmyadmin"`,
  },
  apache: {
    name: "apache",
    program: "/opt/homebrew/opt/httpd/bin/httpd",
    args: `-D FOREGROUND -f "${DATA}/httpd.conf"`,
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
renderSetup();
renderDetected();
setInterval(renderDetected, 4000);
setInterval(refresh, 4000);
