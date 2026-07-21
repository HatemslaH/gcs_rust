/// HTML/CSS/JS страница панели сведения виртуальной RTK-базы.
pub const RTK_CONTROL_PORT: u16 = 5783;

pub const RTK_CONTROL_PAGE_HTML: &str = r#"
<!DOCTYPE html>
<html lang="ru">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>RTK-база — эмулятор</title>
<style>
  :root {
    --bg: #12141a;
    --panel: #1b1f2a;
    --panel-2: #252a38;
    --accent: #3db8ff;
    --accent-2: #ffb020;
    --text: #e8eaf0;
    --text-dim: #9aa3b5;
    --ok: #3ecf8e;
    --warn: #ff6b6b;
    --border: #333a4d;
  }
  * { box-sizing: border-box; }
  body {
    margin: 0;
    background: var(--bg);
    color: var(--text);
    font-family: Segoe UI, system-ui, sans-serif;
    padding: 18px 22px 40px;
  }
  h1 { font-size: 18px; margin: 0 0 4px; }
  .subtitle { color: var(--text-dim); font-size: 13px; margin: 0 0 16px; }
  .status-bar { display: flex; align-items: center; gap: 10px; margin-bottom: 16px; font-size: 13px; }
  .dot { width: 9px; height: 9px; border-radius: 50%; background: var(--warn); flex: none; }
  .dot.online { background: var(--ok); }
  .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 14px; }
  .panel {
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 14px;
  }
  .panel h2 {
    font-size: 12px; text-transform: uppercase; letter-spacing: 0.05em;
    color: var(--text-dim); margin: 0 0 12px;
  }
  .row { display: flex; align-items: center; gap: 8px; margin-bottom: 10px; min-width: 0; }
  .row label { width: 88px; flex: none; font-size: 12px; color: var(--text-dim); }
  .row input[type=range] { flex: 1 1 auto; min-width: 0; width: 100%; }
  .row input[type=number] {
    width: 88px; flex: none; min-width: 0; background: var(--panel-2); border: 1px solid var(--border);
    color: var(--text); border-radius: 6px; padding: 5px 6px; font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .row input[type=number].compact { width: 72px; text-align: right; }
  .hint { font-size: 11px; color: var(--text-dim); margin: -2px 0 10px 96px; }
  .metrics { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; font-size: 13px; }
  .metric {
    background: var(--panel-2); border-radius: 8px; padding: 10px;
    border: 1px solid var(--border);
  }
  .metric .k { color: var(--text-dim); font-size: 11px; }
  .metric .v { font-size: 16px; margin-top: 4px; font-variant-numeric: tabular-nums; }
  .btns { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 4px; }
  button.ctrl {
    background: var(--panel-2); color: var(--text); border: 1px solid var(--border);
    border-radius: 8px; padding: 8px 12px; cursor: pointer; font-size: 12px;
  }
  button.ctrl:hover { border-color: var(--accent); }
  button.ctrl.primary { background: var(--accent); color: #041018; border-color: var(--accent); font-weight: 600; }
  button.ctrl.danger { border-color: var(--warn); color: var(--warn); }
  .log {
    height: 180px; overflow: auto; background: var(--panel-2); border-radius: 8px;
    border: 1px solid var(--border); padding: 8px 10px; font-size: 12px;
    font-family: ui-monospace, Consolas, monospace; color: var(--text-dim);
  }
  .log div { margin-bottom: 4px; }
  .badge {
    display: inline-block; padding: 2px 8px; border-radius: 999px; font-size: 11px;
    background: var(--panel-2); border: 1px solid var(--border);
  }
  .badge.ok { color: var(--ok); border-color: var(--ok); }
  .badge.warn { color: var(--accent-2); border-color: var(--accent-2); }
</style>
</head>
<body>
  <h1>Эмулятор RTK-базы</h1>
  <p class="subtitle">Сведение, FIXED и RTCM · панель синхронизируется по WebSocket</p>
  <div class="status-bar">
    <span class="dot" id="wsDot"></span>
    <span id="wsLabel">подключение…</span>
    <span class="badge" id="statusBadge">—</span>
  </div>

  <div class="grid">
    <div class="panel">
      <h2>Позиция базы</h2>
      <div class="row"><label>Широта</label><input type="number" id="lat" step="0.000001" style="width:130px" /></div>
      <div class="row"><label>Долгота</label><input type="number" id="lon" step="0.000001" style="width:130px" /></div>
      <div class="row"><label>Высота, м</label><input type="number" id="h" step="0.01" style="width:130px" /></div>
      <div class="btns">
        <button class="ctrl primary" id="applyPos">Применить позицию</button>
      </div>
    </div>

    <div class="panel">
      <h2>Сведение</h2>
      <div class="row">
        <label>Качество</label>
        <input type="range" id="quality" min="0" max="100" value="70" />
        <input type="number" class="compact" id="qualityNum" min="0" max="1" step="0.01" value="0.70" />
      </div>
      <div class="row">
        <label>meanAcc, м</label>
        <input type="range" id="meanAcc" min="1" max="1000" value="200" />
        <input type="number" class="compact" id="meanAccNum" min="0.01" max="100" step="0.01" value="2.00" />
      </div>
      <div class="hint">Можно ввести точность вручную в поле справа</div>
      <div class="row">
        <label></label>
        <label style="width:auto;display:flex;align-items:center;gap:8px;color:var(--text-dim);font-size:12px;">
          <input type="checkbox" id="overrideAcc" /> ручной meanAcc
        </label>
      </div>
      <div class="btns">
        <button class="ctrl primary" id="forceValid">Force valid</button>
        <button class="ctrl danger" id="forceFail">Force fail</button>
        <button class="ctrl" id="clearForce">Снять force</button>
        <button class="ctrl" id="resetSurvey">Reset</button>
      </div>
    </div>

    <div class="panel">
      <h2>Телеметрия</h2>
      <div class="metrics">
        <div class="metric"><div class="k">dur</div><div class="v" id="mDur">0 s</div></div>
        <div class="metric"><div class="k">obs</div><div class="v" id="mObs">0</div></div>
        <div class="metric"><div class="k">meanAcc</div><div class="v" id="mAcc">—</div></div>
        <div class="metric"><div class="k">minDur / accLimit</div><div class="v" id="mLimits">—</div></div>
        <div class="metric"><div class="k">active / valid</div><div class="v" id="mFlags">—</div></div>
        <div class="metric"><div class="k">mode</div><div class="v" id="mMode">—</div></div>
      </div>
    </div>

    <div class="panel" style="grid-column: 1 / -1;">
      <h2>Лог</h2>
      <div class="log" id="log"></div>
    </div>
  </div>

<script>
(() => {
  const wsDot = document.getElementById('wsDot');
  const wsLabel = document.getElementById('wsLabel');
  const statusBadge = document.getElementById('statusBadge');
  let ws;
  let applying = false;

  function connect() {
    const proto = location.protocol === 'https:' ? 'wss' : 'ws';
    ws = new WebSocket(proto + '://' + location.host + '/ws');
    ws.onopen = () => {
      wsDot.classList.add('online');
      wsLabel.textContent = 'WebSocket online';
    };
    ws.onclose = () => {
      wsDot.classList.remove('online');
      wsLabel.textContent = 'переподключение…';
      setTimeout(connect, 1000);
    };
    ws.onmessage = (ev) => {
      try {
        const msg = JSON.parse(ev.data);
        if (msg.type === 'state') applyState(msg);
      } catch (_) {}
    };
  }

  function send(obj) {
    if (ws && ws.readyState === 1) ws.send(JSON.stringify(obj));
  }

  function setQualityUi(v) {
    const q = Math.max(0, Math.min(1, Number(v) || 0));
    document.getElementById('quality').value = Math.round(q * 100);
    document.getElementById('qualityNum').value = q.toFixed(2);
    return q;
  }

  function setMeanAccUi(v) {
    const acc = Math.max(0.01, Number(v) || 0.01);
    document.getElementById('meanAcc').value = Math.min(1000, Math.max(1, Math.round(acc * 100)));
    document.getElementById('meanAccNum').value = acc.toFixed(2);
    return acc;
  }

  function applyState(s) {
    applying = true;
    statusBadge.textContent = s.status || '—';
    statusBadge.className = 'badge ' + ((s.valid || s.status === 'fixed') ? 'ok' : (s.active ? 'warn' : ''));
    document.getElementById('lat').value = Number(s.latitude).toFixed(7);
    document.getElementById('lon').value = Number(s.longitude).toFixed(7);
    document.getElementById('h').value = Number(s.heightMsl).toFixed(2);
    const qualityFocused = document.activeElement === document.getElementById('qualityNum');
    const accFocused = document.activeElement === document.getElementById('meanAccNum');
    if (!qualityFocused) setQualityUi(s.surveyQuality ?? 0.7);
    if (!accFocused) setMeanAccUi(s.meanAcc ?? 2);
    document.getElementById('overrideAcc').checked = s.meanAccOverride != null;
    document.getElementById('mDur').textContent = (s.dur ?? 0) + ' s';
    document.getElementById('mObs').textContent = String(s.obs ?? 0);
    document.getElementById('mAcc').textContent = Number(s.meanAcc).toFixed(3) + ' m';
    document.getElementById('mLimits').textContent = (s.minDur ?? 0) + ' s / ' + Number(s.accLimit).toFixed(2) + ' m';
    document.getElementById('mFlags').textContent = (s.active ? '1' : '0') + ' / ' + (s.valid ? '1' : '0');
    document.getElementById('mMode').textContent = s.mode || '—';
    const log = document.getElementById('log');
    log.innerHTML = (s.log || []).map(line => '<div>' + line + '</div>').join('');
    applying = false;
  }

  document.getElementById('quality').addEventListener('input', (e) => {
    if (applying) return;
    const v = setQualityUi(Number(e.target.value) / 100);
    send({ type: 'set', surveyQuality: v });
  });

  document.getElementById('qualityNum').addEventListener('change', (e) => {
    if (applying) return;
    const v = setQualityUi(e.target.value);
    send({ type: 'set', surveyQuality: v });
  });

  document.getElementById('meanAcc').addEventListener('input', (e) => {
    if (applying) return;
    const v = setMeanAccUi(Number(e.target.value) / 100);
    document.getElementById('overrideAcc').checked = true;
    send({ type: 'set', meanAccOverride: v });
  });

  document.getElementById('meanAccNum').addEventListener('change', (e) => {
    if (applying) return;
    const v = setMeanAccUi(e.target.value);
    document.getElementById('overrideAcc').checked = true;
    send({ type: 'set', meanAccOverride: v });
  });

  document.getElementById('overrideAcc').addEventListener('change', (e) => {
    if (e.target.checked) {
      const v = setMeanAccUi(document.getElementById('meanAccNum').value);
      send({ type: 'set', meanAccOverride: v });
    } else {
      send({ type: 'set', meanAccOverride: null });
    }
  });

  document.getElementById('applyPos').onclick = () => send({
    type: 'set',
    latitude: Number(document.getElementById('lat').value),
    longitude: Number(document.getElementById('lon').value),
    heightMsl: Number(document.getElementById('h').value),
  });

  document.getElementById('forceValid').onclick = () => send({ type: 'cmd', cmd: 'forceValid' });
  document.getElementById('forceFail').onclick = () => send({ type: 'cmd', cmd: 'forceFail' });
  document.getElementById('clearForce').onclick = () => send({ type: 'cmd', cmd: 'clearForce' });
  document.getElementById('resetSurvey').onclick = () => send({ type: 'cmd', cmd: 'reset' });

  connect();
})();
</script>
</body>
</html>
"#;
