// Toran dashboard JS. No build step, no deps. Vanilla.
window.toran = (function () {
  function el(html) { var d = document.createElement('div'); d.innerHTML = html.trim(); return d.firstChild; }
  function fmtTime(iso) { try { return new Date(iso).toLocaleString(); } catch (_) { return iso; } }
  function riskClass(score) { if (score >= 70) return 'high'; if (score >= 40) return 'med'; return 'low'; }

  async function getJSON(url) {
    var r = await fetch(url);
    if (!r.ok) throw new Error('http ' + r.status);
    return r.json();
  }
  async function postJSON(url, body) {
    var r = await fetch(url, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) });
    return r.json();
  }

  function homeStatus() {
    getJSON('/api/health').then(function (j) {
      var s = document.getElementById('home-status');
      s.innerHTML = '<p>Status: <b>' + j.status + '</b> · Pending: ' + j.pending_approvals + ' · Default action: ' + j.default_action + '</p><p>Socket: <code>' + j.socket + '</code></p>';
    }).catch(function (e) { document.getElementById('home-status').textContent = 'health error: ' + e; });
  }

  function startDashboard() {
    async function tick() {
      try {
        var s = await getJSON('/api/summary');
        document.getElementById('m-pending').textContent = s.pending;
        document.getElementById('m-decisions').textContent = s.total_decisions;
        document.getElementById('m-avg').textContent = s.avg_eval_ns;
        document.getElementById('m-uptime').textContent = s.uptime_secs;
        var q = await getJSON('/api/approvals?status=pending&limit=200');
        var rows = document.getElementById('rows');
        var empty = document.getElementById('empty');
        rows.innerHTML = '';
        var arr = q.approvals || [];
        empty.style.display = arr.length ? 'none' : 'block';
        arr.forEach(function (r) {
          var tr = document.createElement('tr');
          tr.innerHTML = '<td>' + fmtTime(r.created_at) + '</td>'
            + '<td><code>' + escape(r.function_name) + '</code></td>'
            + '<td>' + escape(r.agent_id) + '</td>'
            + '<td><span class="tag-risk ' + riskClass(r.risk_score) + '">' + r.risk_score + '</span></td>'
            + '<td><code>' + escape(r.policy_rule) + '</code></td>'
            + '<td><a href="/dashboard/approval/' + r.id + '">Open</a></td>';
          rows.appendChild(tr);
        });
      } catch (e) {
        console.error('tick', e);
      }
    }
    tick();
    setInterval(tick, 2000);
  }

  async function loadDetail(id) {
    try {
      var j = await getJSON('/api/approvals/' + id);
      var r = j.approval;
      var args = JSON.parse(r.arguments_json || '{}');
      var ctx = JSON.parse(r.context_json || '{}');
      var root = document.getElementById('detail');
      root.innerHTML = '<p><b>Function:</b> <code>' + escape(r.function_name) + '</code> · <b>Agent:</b> ' + escape(r.agent_id) + ' · <b>Status:</b> ' + r.status + '</p>'
        + '<p><b>Policy rule:</b> <code>' + escape(r.policy_rule) + '</code> · <b>Risk:</b> <span class="tag-risk ' + riskClass(r.risk_score) + '">' + r.risk_score + '</span></p>'
        + '<h3>Arguments</h3><pre>' + escape(JSON.stringify(args, null, 2)) + '</pre>'
        + '<h3>Context</h3><pre>' + escape(JSON.stringify(ctx, null, 2)) + '</pre>'
        + '<h3>Notify token</h3><pre>' + escape(r.notify_token) + '</pre>';
      window.__lastToken = r.notify_token;
    } catch (e) { document.getElementById('detail').textContent = 'error: ' + e; }
  }

  async function resolve(id, op) {
    var token = window.__lastToken || prompt('notify token?');
    if (!token) return;
    var url = '/api/approvals/' + id + '/' + op;
    try {
      var j = await postJSON(url, { resolved_by: 'dashboard', token: token });
      if (j.ok) { alert('resolved: ' + j.approval.status); location.href = '/dashboard'; }
      else alert('error: ' + JSON.stringify(j));
    } catch (e) { alert('error: ' + e); }
  }

  function startAudit() {
    async function tick() {
      try {
        var j = await getJSON('/api/audit?limit=200');
        var rows = document.getElementById('rows');
        rows.innerHTML = '';
        (j.audit || []).forEach(function (r) {
          var tr = document.createElement('tr');
          tr.innerHTML = '<td>' + fmtTime(r.timestamp) + '</td>'
            + '<td>' + escape(r.event_type) + '</td>'
            + '<td><code>' + escape(r.function_name) + '</code></td>'
            + '<td>' + escape(r.agent_id) + '</td>'
            + '<td>' + escape(r.decision) + '</td>'
            + '<td><code>' + escape(r.policy_rule) + '</code></td>';
          rows.appendChild(tr);
        });
      } catch (e) { console.error(e); }
    }
    tick();
    setInterval(tick, 3000);
  }

  function startPolicies() {
    getJSON('/api/policies').then(function (j) {
      var root = document.getElementById('policies');
      var html = '';
      (j.policies || []).forEach(function (p) {
        html += '<div class="policy-card"><h3>' + escape(p.name) + '</h3><div class="meta">' + p.rule_count + ' rules</div></div>';
      });
      (j.files || []).forEach(function (f) {
        html += '<div class="policy-card"><h3><a href="/api/policies/' + encodeURIComponent(f.name) + '">' + escape(f.name) + '</a></h3><div class="meta">' + escape(f.path) + '</div></div>';
      });
      root.innerHTML = html || '<p class="empty">No policies found in <code>./policies</code>.</p>';
    }).catch(function (e) {
      document.getElementById('policies').textContent = 'error: ' + e;
    });
  }

  function escape(s) { return String(s).replace(/[&<>"']/g, function (c) { return ({ '&':'&amp;', '<':'&lt;', '>':'&gt;', '"':'&quot;', "'":'&#39;' })[c]; }); }

  // -------- Tour (driver.js) --------
  var tour = {
    steps: function (page) {
      var home = [
        { element: 'header h1', popover: { title: 'Welcome to Toran', description: 'Toran is a human-approval gatekeeper for AI agents. When your agent tries to call a risky tool, Toran pauses it and waits for a human to approve or deny.' } },
        { element: 'header nav', popover: { title: 'Navigation', description: 'These links take you to the approval queue, audit log, policies, and the live API endpoints. Every page has its own tour.' } },
        { element: '#home-status', popover: { title: 'Live status', description: 'This card shows the live status of the server: default action, pending count, and the socket path the Python SDK connects to.' } },
        { element: 'a[href="/dashboard"]', popover: { title: 'Try the queue', description: 'Click here to see the approval queue. If there are no pending approvals, you can create one from the CLI: <code>curl -X POST http://127.0.0.1:7878/api/approvals -d \'&#123;...&#125;\' -H content-type:application/json</code>' } }
      ];
      var queue = [
        { element: '#cards', popover: { title: 'Live metrics', description: 'Four numbers, polled every 2 seconds: pending count, total decisions, average evaluation latency in nanoseconds, and uptime. Avg eval < 1ms is the design target.' } },
        { element: '#queue-section', popover: { title: 'The approval queue', description: 'Each row is a single tool call that hit a REQUIRE_APPROVAL rule. Click <b>Open</b> to see the arguments and the resolve buttons.' } },
        { element: 'nav a[href="/dashboard/audit"]', popover: { title: 'Audit log', description: 'Every decision (allow, block, approve, deny, timeout) is written here. Append-only SQLite, never modified after the fact.' } },
        { element: 'nav a[href="/dashboard/policies"]', popover: { title: 'Policies', description: 'Browse the loaded YAML files. Edit them in <code>./policies/</code> and the server hot-reloads automatically.' } }
      ];
      var detail = [
        { element: '#detail', popover: { title: 'Full approval record', description: 'Function name, agent, risk score, the policy rule that fired, the JSON arguments, the JSON context, and the notify token.' } },
        { element: '#btn-approve', popover: { title: 'Approve', description: 'Approving writes status=APPROVED to SQLite and wakes up the waiting Python SDK. The original function then runs.' } },
        { element: '#btn-deny', popover: { title: 'Deny', description: 'Denying raises a ToranDeniedError in the Python SDK. The original function never runs.' } }
      ];
      var audit = [
        { element: '#audit-table', popover: { title: 'Append-only audit log', description: 'Every decision lands here. Event type, function, agent, decision, rule. Polled every 3 seconds. The JSON form is at <code>/api/audit?limit=500</code>.' } }
      ];
      var policies = [
        { element: '#policies', popover: { title: 'Loaded policies', description: 'Each card is a YAML file. Click the file name to see the raw YAML. The server watches this directory and hot-reloads on every save.' } }
      ];
      var map = { home: home, queue: queue, detail: detail, audit: audit, policies: policies };
      return map[page] || home;
    },
    run: function (page) {
      var factory = window.driver && window.driver.js && window.driver.js.driver;
      if (typeof factory !== 'function') return;
      var d = factory({ showProgress: true, animate: true });
      d.setSteps(this.steps(page));
      d.drive();
    },
    bind: function (page) {
      var self = this;
      var link = document.getElementById('start-tour');
      if (link) link.addEventListener('click', function (e) { e.preventDefault(); self.run(page); });
      var seen = (typeof localStorage !== 'undefined') && localStorage.getItem('toran.tour.' + page);
      if (!seen) setTimeout(function () { self.run(page); localStorage.setItem('toran.tour.' + page, '1'); }, 600);
    },
    bindHome: function () { this.bind('home'); },
    bindDashboard: function () { this.bind('queue'); },
    bindDetail: function () { this.bind('detail'); },
    bindAudit: function () { this.bind('audit'); },
    bindPolicies: function () { this.bind('policies'); }
  };

  return { homeStatus: homeStatus, startDashboard: startDashboard, loadDetail: loadDetail, resolve: resolve, startAudit: startAudit, startPolicies: startPolicies, tour: tour };
})();
