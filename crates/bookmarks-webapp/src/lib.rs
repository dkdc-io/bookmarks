//! Embedded webapp for bookmarks.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Html;
use axum::routing::{get, post};
use axum::{Json, Router};
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tokio::sync::oneshot;

use bookmarks_core::config::{Config, UrlEntry};
use bookmarks_core::storage::Storage;
use bookmarks_core::strings;

const DEFAULT_WEBAPP_PORT: u16 = 1414;

fn default_webapp_addr() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], DEFAULT_WEBAPP_PORT))
}

struct AppState {
    storage: Mutex<Box<dyn Storage>>,
}

impl AppState {
    fn lock_storage(&self) -> std::sync::MutexGuard<'_, Box<dyn Storage>> {
        self.storage.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn load_config(&self) -> Result<Config, String> {
        self.lock_storage().load().map_err(|e| e.to_string())
    }

    fn storage_metadata(&self) -> (String, Option<String>) {
        let storage = self.lock_storage();
        (
            storage.backend_name().to_string(),
            storage
                .path()
                .map(|path| path.to_string_lossy().into_owned()),
        )
    }

    /// Hold the lock across the entire load-modify-save cycle to prevent
    /// TOCTOU races between concurrent requests.
    fn modify_config<F>(&self, f: F) -> Result<(), String>
    where
        F: FnOnce(&mut Config) -> Result<(), String>,
    {
        let storage = self.lock_storage();
        let mut config = storage.load().map_err(|e| e.to_string())?;
        f(&mut config)?;
        storage.save(&config).map_err(|e| e.to_string())
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn js_attr_string(s: &str) -> String {
    let literal = serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string());
    escape(&literal)
}

// -- HTML rendering ----------------------------------------------------------

const STYLE: &str = r#"
* { margin: 0; padding: 0; box-sizing: border-box; }
html { background: #1a1a29; }
body { font-family: system-ui, -apple-system, sans-serif; background: #1a1a29; color: #8c8ca6; max-width: 720px; margin: 0 auto; padding: 32px 24px; }
h1 { font-size: 1.4rem; color: #8c8ca6; margin-bottom: 8px; font-weight: 500; }
.subtitle { font-size: 0.85rem; color: #8c8ca6; margin-bottom: 24px; }
.subtitle a { color: #bf4dff; text-decoration: none; }
.subtitle a:hover { text-decoration: underline; }
h2 { font-size: 1rem; color: #8c8ca6; margin-bottom: 12px; text-transform: lowercase; }
.section { margin-bottom: 28px; }
table { width: 100%; border-collapse: collapse; table-layout: fixed; }
col.col-check { width: 28px; }
col.col-name { width: 130px; }
col.col-actions { width: 70px; }
th { text-align: left; font-size: 0.75rem; color: #666680; text-transform: uppercase; letter-spacing: 0.05em; padding: 6px 8px; border-bottom: 1px solid #2e2e47; }
th.sortable { cursor: pointer; user-select: none; }
th.sortable:hover { color: #8c8ca6; }
th.active { color: #bf4dff; }
td { padding: 6px 8px; border-bottom: 1px solid #242438; font-size: 0.85rem; vertical-align: top; overflow: hidden; text-overflow: ellipsis; }
td.check, th.check { text-align: center; overflow: visible; }
td.check input, th.check input { cursor: pointer; accent-color: #bf4dff; }
td.name { color: #bf4dff; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
td.name a { color: #bf4dff; text-decoration: none; }
td.name a:hover { text-decoration: underline; }
td.url a { color: #22d3ee; text-decoration: none; word-break: break-all; }
td.url a:hover { text-decoration: underline; color: #67e8f9; }
td.aliases { color: #a640f2; font-size: 0.8rem; }
td.entries a { color: #a640f2; text-decoration: none; }
td.entries a:hover { text-decoration: underline; color: #bf4dff; }
.actions { text-align: right; white-space: nowrap; }
.btn { background: none; border: 1px solid #2e2e47; color: #8c8ca6; padding: 2px 8px; border-radius: 4px; cursor: pointer; font-size: 0.75rem; }
.btn:hover { border-color: #666680; color: #edeedf; }
.btn-danger { border-color: #5c2a2a; color: #ff7373; }
.btn-danger:hover { border-color: #ff7373; color: #ffa0a0; }
.btn-add { background: #242438; border-color: #2e2e47; color: #bf4dff; white-space: nowrap; width: 72px; text-align: center; flex-shrink: 0; }
.btn-add:hover { background: #2e2e47; border-color: #666680; }
.bulk-bar { display: none; align-items: center; gap: 8px; margin-bottom: 12px; padding: 8px 12px; background: #242438; border: 1px solid #2e2e47; border-radius: 6px; }
.bulk-bar.visible { display: flex; }
.bulk-bar .bulk-count { font-size: 0.8rem; color: #bf4dff; }
form.inline { display: flex; gap: 6px; align-items: center; margin-top: 6px; }
form.inline input { background: #242438; border: 1px solid #2e2e47; color: #edeedf; padding: 5px 8px; border-radius: 4px; font-size: 0.8rem; min-width: 0; }
form.inline input:first-of-type { flex: 2; }
form.inline input:nth-of-type(2) { flex: 3; }
form.inline input::placeholder { color: #666680; }
form.inline input:focus { outline: none; border-color: #bf4dff; }
.copy-btn { background: none; border: none; color: #666680; cursor: pointer; padding: 0; line-height: 1; flex-shrink: 0; vertical-align: middle; }
.copy-btn:hover { color: #8c8ca6; }
.copy-btn.copied { color: #4ade80; }
td.url .url-cell { display: flex; align-items: center; gap: 6px; }
.error-banner { background: #3a1a2a; border: 1px solid #5c2a2a; color: #ff7373; padding: 8px 12px; border-radius: 6px; margin-bottom: 12px; font-size: 0.8rem; cursor: pointer; }
.editable { cursor: pointer; }
.editable:hover { background: #2e2e47; border-radius: 3px; }
.edit-input { background: #242438; border: 1px solid #bf4dff; color: #edeedf; padding: 3px 6px; border-radius: 3px; font-size: 0.8rem; width: 100%; font-family: inherit; }
.edit-input:focus { outline: none; }
.empty { color: #666680; font-style: italic; font-size: 0.85rem; padding: 12px 0; }
.toolbar { display: flex; gap: 8px; align-items: center; margin-bottom: 16px; }
.toolbar input { background: #242438; border: 1px solid #2e2e47; color: #edeedf; padding: 5px 8px; border-radius: 4px; font-size: 0.8rem; width: 200px; }
.toolbar input::placeholder { color: #666680; }
.toolbar input:focus { outline: none; border-color: #bf4dff; }
.tabs { display: flex; gap: 4px; flex-shrink: 0; }
.tab { background: none; border: 1px solid #2e2e47; color: #8c8ca6; padding: 4px 10px; border-radius: 4px; cursor: pointer; font-size: 0.75rem; }
.tab:hover { color: #edeedf; border-color: #666680; }
.tab.active { color: #bf4dff; border-color: #bf4dff; background: #382952; }
.counts { font-size: 0.7rem; color: #666680; margin-left: 3px; }
.modal-overlay { display: none; position: fixed; inset: 0; background: rgba(0,0,0,0.7); z-index: 100; align-items: center; justify-content: center; }
.modal-overlay.visible { display: flex; }
.modal { background: #141421; border: 1px solid #2e2e47; border-radius: 8px; padding: 24px; max-width: 400px; width: 90%; }
.modal h3 { color: #edeedf; font-size: 1rem; margin-bottom: 8px; }
.modal p { color: #8c8ca6; font-size: 0.85rem; margin-bottom: 16px; line-height: 1.4; }
.modal .modal-actions { display: flex; gap: 8px; justify-content: flex-end; }
.modal .btn-cancel { border-color: #2e2e47; color: #8c8ca6; padding: 6px 16px; font-size: 0.8rem; }
.modal .btn-confirm { background: #3a1a2a; border-color: #ff7373; color: #ff7373; padding: 6px 16px; font-size: 0.8rem; }
@media (max-width: 680px) {
  body { width: auto; padding: 24px 16px; }
  .toolbar { flex-wrap: wrap; }
  .toolbar input { width: 100%; }
  .tabs { width: 100%; }
  .tab { flex: 1; text-align: center; }
  form.inline { flex-wrap: wrap; }
  form.inline input:first-of-type { flex: 1 1 100%; }
  form.inline input:nth-of-type(2) { flex: 1 1 auto; }
  .btn-add { flex-shrink: 0; }
  col.col-name { width: 100px; }
  col.col-actions { width: 60px; }
}
"#;

const SCRIPT: &str = r#"
var pendingAction = null;

function contentEl() {
  return document.getElementById('content');
}

function afterSwap() {
  updateBulkBar();
  document.querySelectorAll('.select-all').forEach(function(cb) { cb.checked = false; });
}

function swapContent(html) {
  contentEl().innerHTML = html;
  afterSwap();
}

function fetchContent(url) {
  fetch(url, { headers: { 'X-Requested-With': 'fetch' } })
    .then(function(r) { if (!r.ok) throw new Error(r.statusText); return r.text(); })
    .then(swapContent);
}

function postAndSwap(url, body) {
  fetch(url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    body: body || ''
  })
    .then(function(r) { if (!r.ok) throw new Error(r.statusText); return r.text(); })
    .then(swapContent);
}

document.addEventListener('submit', function(e) {
  var form = e.target.closest('form[data-post]');
  if (!form) return;
  e.preventDefault();
  postAndSwap(form.dataset.post, new URLSearchParams(new FormData(form)).toString());
});

document.addEventListener('click', function(e) {
  var trigger = e.target.closest('[data-fetch]');
  if (!trigger) return;
  e.preventDefault();
  fetchContent(trigger.dataset.fetch);
});

function confirmDelete(title, message, action) {
  document.getElementById('confirm-title').textContent = title;
  document.getElementById('confirm-message').textContent = message;
  document.getElementById('confirm-modal').classList.add('visible');
  pendingAction = action;
  document.getElementById('confirm-btn').onclick = function() {
    var action = pendingAction;
    closeModal();
    if (action) action();
  };
}

function closeModal() {
  document.getElementById('confirm-modal').classList.remove('visible');
  pendingAction = null;
}

document.addEventListener('DOMContentLoaded', function() {
  document.getElementById('confirm-modal').addEventListener('click', function(e) {
    if (e.target === this) closeModal();
  });
  document.addEventListener('keydown', function(e) {
    if (e.key === 'Escape') closeModal();
  });
});

function openGroup(urls) {
  urls.forEach(function(u) { window.open(u, '_blank', 'noopener'); });
}

function copyUrl(btn, text) {
  navigator.clipboard.writeText(text).then(function() {
    btn.classList.add('copied');
    btn.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>';
    setTimeout(function() {
      btn.classList.remove('copied');
      btn.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>';
    }, 1500);
  });
}

function startEdit(event, type, name, field, currentValue) {
  var cell = event.target.closest('td');
  if (cell.querySelector('.edit-input')) return;
  var original = cell.innerHTML;
  var done = false;
  var input = document.createElement('input');
  input.className = 'edit-input';
  input.value = currentValue;
  function finish(save) {
    if (done) return;
    done = true;
    if (save && input.value.trim() && input.value !== currentValue) {
      submitEdit(type, name, field, input.value.trim(), cell, original);
    } else {
      cell.innerHTML = original;
    }
  }
  input.addEventListener('keydown', function(e) {
    if (e.key === 'Enter') { e.preventDefault(); finish(true); }
    if (e.key === 'Escape') { finish(false); }
  });
  input.addEventListener('blur', function() { finish(true); });
  cell.innerHTML = '';
  cell.appendChild(input);
  input.focus();
  input.select();
}

function submitEdit(type, name, field, value, cell, original) {
  var params = new URLSearchParams();
  if (field === 'name') params.append('new_name', value);
  if (field === 'url') params.append('new_url', value);
  if (field === 'aliases') params.append('new_aliases', value);
  if (field === 'entries') params.append('new_entries', value);
  fetch('/edit/' + type + '/' + encodeURIComponent(name), {
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    body: params.toString()
  })
    .then(function(r) { if (!r.ok) throw new Error(r.statusText); return r.text(); })
    .then(swapContent)
    .catch(function() { cell.innerHTML = original; });
}

function deleteSingle(type, name) {
  confirmDelete(
    'delete ' + type,
    'are you sure you want to delete ' + type + ' "' + name + '"? this cannot be undone.',
    function() { postAndSwap('/delete/' + type + '/' + encodeURIComponent(name)); }
  );
}

function updateBulkBar() {
  var checked = document.querySelectorAll('input.row-check:checked');
  var bar = document.getElementById('bulk-bar');
  var count = document.getElementById('bulk-count');
  if (!bar || !count) return;
  if (checked.length > 0) {
    bar.classList.add('visible');
    count.textContent = checked.length + ' selected';
  } else {
    bar.classList.remove('visible');
  }
}

function toggleAll(src) {
  document.querySelectorAll('input.row-check').forEach(function(cb) {
    if (cb.closest('tr').style.display !== 'none') cb.checked = src.checked;
  });
  updateBulkBar();
}

function clearSelected() {
  document.querySelectorAll('input.row-check').forEach(function(cb) { cb.checked = false; });
  updateBulkBar();
}

function deleteSelected() {
  var checked = document.querySelectorAll('input.row-check:checked');
  if (checked.length === 0) return;
  var items = [];
  checked.forEach(function(cb) { items.push(cb.dataset.type + ' "' + cb.dataset.name + '"'); });
  confirmDelete(
    'delete ' + checked.length + ' item' + (checked.length > 1 ? 's' : ''),
    'are you sure you want to delete: ' + items.join(', ') + '? this cannot be undone.',
    function() {
      var toDelete = [];
      checked.forEach(function(cb) {
        toDelete.push({ type: cb.dataset.type, name: cb.dataset.name });
      });
      var i = 0;
      function next() {
        if (i >= toDelete.length) {
          fetchContent('/content');
          return;
        }
        var item = toDelete[i++];
        fetch('/delete/' + item.type + '/' + encodeURIComponent(item.name), { method: 'POST' }).then(next);
      }
      next();
    }
  );
}

function filterRows() {
  var q = document.getElementById('search').value.toLowerCase();
  document.querySelectorAll('table tr[data-filter]').forEach(function(row) {
    row.style.display = row.getAttribute('data-filter').toLowerCase().includes(q) ? '' : 'none';
  });
}

function showTab(tab) {
  ['urls','groups'].forEach(function(t) {
    var el = document.getElementById('section-' + t);
    var btn = document.getElementById('tab-' + t);
    if (el) el.style.display = (t === tab || tab === 'all') ? '' : 'none';
    if (btn) btn.classList.toggle('active', t === tab);
  });
  var allBtn = document.getElementById('tab-all');
  if (allBtn) allBtn.classList.toggle('active', tab === 'all');
  filterRows();
}
"#;

fn page(body: &str) -> String {
    let project_url = strings::PROJECT_URL;
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>bookmarks</title>
  <style>{STYLE}</style>
</head>
<body>
  <h1>Bookmarks</h1>
  <p class="subtitle"><a href="{project_url}" target="_blank" rel="noopener">bookmarks</a> in your filesystem</p>
  <div id="content">
    {body}
  </div>
  <div class="modal-overlay" id="confirm-modal">
    <div class="modal">
      <h3 id="confirm-title">confirm delete</h3>
      <p id="confirm-message"></p>
      <div class="modal-actions">
        <button class="btn btn-cancel" onclick="closeModal()">cancel</button>
        <button class="btn btn-confirm" id="confirm-btn">delete</button>
      </div>
    </div>
  </div>
  <script>{SCRIPT}</script>
</body>
</html>"##
    )
}

fn resolve_url<'a>(name: &str, config: &'a Config) -> Option<&'a str> {
    bookmarks_core::open::resolve_uri(name, config).ok()
}

fn linked_name(name: &str, url: &str) -> String {
    let n = escape(name);
    let u = escape(url);
    format!(r##"<a href="{u}" target="_blank" rel="noopener" title="{u}">{n}</a>"##)
}

fn copy_btn(url: &str) -> String {
    let uj = js_attr_string(url);
    format!(
        r##"<button class="copy-btn" onclick="copyUrl(this,{uj})" title="copy to clipboard"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg></button>"##
    )
}

fn url_row(name: &str, entry: &UrlEntry) -> String {
    let n = escape(name);
    let nj = js_attr_string(name);
    let url = entry.url();
    let u = escape(url);
    let uj = js_attr_string(url);
    let name_link = linked_name(name, url);
    let copy = copy_btn(url);
    let aliases = entry.aliases();
    let aliases_raw = aliases.join(", ");
    let aliases_raw_js = js_attr_string(&aliases_raw);
    let aliases_html = if aliases.is_empty() {
        r#"<span style="color:#666680;font-size:0.8rem;font-style:italic">+ aliases</span>"#
            .to_string()
    } else {
        let escaped: Vec<String> = aliases.iter().map(|a| escape(a)).collect();
        format!(
            r#"<span style="color:#a640f2;font-size:0.8rem">{}</span>"#,
            escaped.join(", ")
        )
    };
    format!(
        r##"<tr data-filter="{n} {u} {aliases_filter}">
  <td class="check"><input type="checkbox" class="row-check" data-type="url" data-name="{n}" onchange="updateBulkBar()"></td>
  <td class="name editable" ondblclick="startEdit(event,'url',{nj},'name',{nj})">{name_link}</td>
  <td class="url editable" ondblclick="startEdit(event,'url',{nj},'url',{uj})"><span class="url-cell">{copy}<a href="{u}" target="_blank" rel="noopener">{u}</a></span></td>
  <td class="aliases editable" ondblclick="startEdit(event,'url',{nj},'aliases',{aliases_raw_js})">{aliases_html}</td>
  <td class="actions">
    <button class="btn btn-danger" onclick="deleteSingle('url',{nj})">delete</button>
  </td>
</tr>"##,
        aliases_filter = escape(&aliases.join(" ")),
    )
}

fn group_row(name: &str, entries: &[String], config: &Config) -> String {
    let n = escape(name);
    let nj = js_attr_string(name);
    let urls: Vec<String> = entries
        .iter()
        .filter_map(|entry| resolve_url(entry, config).map(js_attr_string))
        .collect();
    let urls_arr = urls.join(",");
    let entry_links: Vec<String> = entries
        .iter()
        .map(|entry| {
            let e = escape(entry);
            if let Some(url) = resolve_url(entry, config) {
                let u = escape(url);
                format!(r##"<a href="{u}" target="_blank" rel="noopener" title="{u}">{e}</a>"##)
            } else {
                e
            }
        })
        .collect();
    let entries_html = entry_links.join(", ");
    let filter_str = entries
        .iter()
        .map(|e| escape(e))
        .collect::<Vec<_>>()
        .join(", ");
    let name_cell = if urls.is_empty() {
        n.clone()
    } else {
        format!(
            r##"<a href="#" onclick="openGroup([{urls_arr}]);return false;" title="open all {count} urls">{n}</a>"##,
            count = urls.len()
        )
    };
    let entries_raw_js = js_attr_string(&entries.join(", "));
    format!(
        r##"<tr data-filter="{n} {filter_str}">
  <td class="check"><input type="checkbox" class="row-check" data-type="group" data-name="{n}" onchange="updateBulkBar()"></td>
  <td class="name editable" ondblclick="startEdit(event,'group',{nj},'name',{nj})">{name_cell}</td>
  <td class="entries editable" ondblclick="startEdit(event,'group',{nj},'entries',{entries_raw_js})">{entries_html}</td>
  <td class="actions">
    <button class="btn btn-danger" onclick="deleteSingle('group',{nj})">delete</button>
  </td>
</tr>"##
    )
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SortField {
    Name,
    Url,
}

fn render_content(config: &Config, sort: SortField, error: Option<&str>) -> String {
    let mut urls: Vec<_> = config.urls.iter().collect();
    let mut groups: Vec<_> = config.groups.iter().collect();

    match sort {
        SortField::Name => {
            urls.sort_by_key(|(k, _)| k.as_str());
            groups.sort_by_key(|(k, _)| k.as_str());
        }
        SortField::Url => {
            urls.sort_by_key(|(_, v)| v.url());
            groups.sort_by_key(|(k, _)| k.as_str());
        }
    }

    let name_cls = if sort == SortField::Name {
        " active"
    } else {
        ""
    };
    let url_cls = if sort == SortField::Url {
        " active"
    } else {
        ""
    };

    let mut html = String::new();

    html.push_str(&format!(
        r##"<div class="toolbar">
  <input id="search" type="text" placeholder="{ph_filter}" oninput="filterRows()" autocomplete="off">
  <div class="tabs">
    <button id="tab-all" class="tab active" onclick="showTab('all')">all</button>
    <button id="tab-urls" class="tab" onclick="showTab('urls')">urls<span class="counts">{uc}</span></button>
    <button id="tab-groups" class="tab" onclick="showTab('groups')">groups<span class="counts">{gc}</span></button>
  </div>
</div>"##,
        ph_filter = strings::PH_FILTER,
        uc = urls.len(),
        gc = groups.len(),
    ));

    if let Some(msg) = error {
        let m = escape(msg);
        html.push_str(&format!(
            r##"<div class="error-banner" onclick="this.remove()">{m} <span style="margin-left:8px;cursor:pointer;opacity:0.6">x</span></div>"##
        ));
    }

    html.push_str(
        r##"<div class="bulk-bar" id="bulk-bar">
  <span class="bulk-count" id="bulk-count">0 selected</span>
  <button class="btn btn-danger" onclick="deleteSelected()">delete selected</button>
  <button class="btn" onclick="clearSelected()">clear</button>
</div>"##,
    );

    html.push_str(&format!(
        r##"<div class="section">
<form class="inline" data-post="/add/url" action="/add/url" method="post">
  <input name="name" placeholder="{ph_url_name}" required>
  <input name="url" placeholder="{ph_url}" required>
  <button class="btn btn-add" type="submit">+ url</button>
</form>
<form class="inline" data-post="/add/group" action="/add/group" method="post">
  <input name="name" placeholder="{ph_group_name}" required>
  <input name="entries" placeholder="{ph_group_entries}" required>
  <button class="btn btn-add" type="submit">+ group</button>
</form>
</div>"##,
        ph_url_name = strings::PH_URL_NAME,
        ph_url = strings::PH_URL,
        ph_group_name = strings::PH_GROUP_NAME,
        ph_group_entries = strings::PH_GROUP_ENTRIES,
    ));

    html.push_str(r##"<div class="section" id="section-urls"><h2>urls</h2>"##);
    if urls.is_empty() {
        html.push_str(r#"<p class="empty">no urls yet</p>"#);
    } else {
        html.push_str(&format!(
            r##"<table><colgroup><col class="col-check"><col class="col-name"><col class="col-value"><col style="width:120px"><col class="col-actions"></colgroup><tr><th class="check"><input type="checkbox" class="select-all" onchange="toggleAll(this)"></th><th class="sortable{name_cls}" data-fetch="/content?sort=name">name</th><th class="sortable{url_cls}" data-fetch="/content?sort=url">url</th><th>aliases</th><th></th></tr>"##,
        ));
        for (name, entry) in &urls {
            html.push_str(&url_row(name, entry));
        }
        html.push_str("</table>");
    }
    html.push_str("</div>");

    html.push_str(r##"<div class="section" id="section-groups"><h2>groups</h2>"##);
    if groups.is_empty() {
        html.push_str(r#"<p class="empty">no groups yet</p>"#);
    } else {
        html.push_str(&format!(
            r##"<table><colgroup><col class="col-check"><col class="col-name"><col class="col-value"><col class="col-actions"></colgroup><tr><th class="check"><input type="checkbox" class="select-all" onchange="toggleAll(this)"></th><th class="sortable{name_cls}" data-fetch="/content?sort=name">group</th><th>entries</th><th></th></tr>"##,
        ));
        for (name, entries) in &groups {
            html.push_str(&group_row(name, entries, config));
        }
        html.push_str("</table>");
    }
    html.push_str("</div>");

    html
}

// -- Handlers ----------------------------------------------------------------

type S = State<Arc<AppState>>;
type Form = axum::extract::Form<std::collections::HashMap<String, String>>;

#[derive(Debug, serde::Deserialize, Default)]
struct ContentQuery {
    #[serde(default)]
    sort: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct HealthResponse {
    status: &'static str,
    app: &'static str,
    backend: String,
    path: Option<String>,
}

fn parse_sort(q: &ContentQuery) -> SortField {
    match q.sort.as_deref() {
        Some("url") => SortField::Url,
        _ => SortField::Name,
    }
}

async fn health(State(state): S) -> Json<HealthResponse> {
    let (backend, path) = state.storage_metadata();
    Json(HealthResponse {
        status: "ok",
        app: "bookmarks-webapp",
        backend,
        path,
    })
}

async fn favicon() -> StatusCode {
    StatusCode::NO_CONTENT
}

async fn index(State(state): S, q: Query<ContentQuery>) -> Html<String> {
    Html(page(&render_state_content(&state, parse_sort(&q), None)))
}

async fn content(State(state): S, q: Query<ContentQuery>) -> Html<String> {
    Html(render_state_content(&state, parse_sort(&q), None))
}

fn content_ok(state: &Arc<AppState>) -> Html<String> {
    Html(render_state_content(state, SortField::Name, None))
}

fn content_err(state: &Arc<AppState>, msg: &str) -> Html<String> {
    Html(render_state_content(state, SortField::Name, Some(msg)))
}

fn render_state_content(state: &Arc<AppState>, sort: SortField, error: Option<&str>) -> String {
    match state.load_config() {
        Ok(config) => render_content(&config, sort, error),
        Err(load_err) => {
            let message = match error {
                Some(error) => format!("{error}; failed to load config: {load_err}"),
                None => format!("failed to load config: {load_err}"),
            };
            render_content(&Config::default(), sort, Some(&message))
        }
    }
}

fn modify_or_err(
    state: &Arc<AppState>,
    f: impl FnOnce(&mut Config) -> Result<(), String>,
) -> Html<String> {
    match state.modify_config(f) {
        Ok(()) => content_ok(state),
        Err(e) => content_err(state, &e),
    }
}

async fn add_url(State(state): S, axum::extract::Form(form): Form) -> Html<String> {
    let name = form.get("name").cloned().unwrap_or_default();
    let url = form.get("url").cloned().unwrap_or_default();
    if !name.is_empty() && !url.is_empty() {
        return modify_or_err(&state, |config| {
            config.upsert_url(name, url);
            Ok(())
        });
    }
    content_ok(&state)
}

async fn add_group(State(state): S, axum::extract::Form(form): Form) -> Html<String> {
    let name = form.get("name").cloned().unwrap_or_default();
    let entries_raw = form.get("entries").cloned().unwrap_or_default();
    if !name.is_empty() && !entries_raw.is_empty() {
        let entries = Config::parse_list(&entries_raw);
        if !entries.is_empty() {
            return modify_or_err(&state, |config| {
                config
                    .upsert_group(name, entries)
                    .map_err(|e| e.to_string())
            });
        }
    }
    content_ok(&state)
}

async fn delete_url(State(state): S, Path(name): Path<String>) -> Html<String> {
    modify_or_err(&state, |config| {
        config.delete_url(&name).map_err(|e| e.to_string())
    })
}

async fn delete_group(State(state): S, Path(name): Path<String>) -> Html<String> {
    modify_or_err(&state, |config| {
        config.delete_group(&name).map_err(|e| e.to_string())
    })
}

async fn edit_url(
    State(state): S,
    Path(name): Path<String>,
    axum::extract::Form(form): Form,
) -> Html<String> {
    let new_name = form.get("new_name").filter(|s| !s.is_empty()).cloned();
    let new_url = form.get("new_url").filter(|s| !s.is_empty()).cloned();
    let new_aliases = form.get("new_aliases").cloned();

    modify_or_err(&state, |config| {
        let key = if let Some(ref new_name) = new_name
            && new_name != &name
        {
            config
                .rename_url(&name, new_name)
                .map_err(|e| e.to_string())?;
            new_name.clone()
        } else {
            name
        };

        if let Some(new_url) = new_url {
            config
                .set_url_value(&key, new_url)
                .map_err(|e| e.to_string())?;
        }

        if let Some(new_aliases) = new_aliases {
            config
                .set_url_aliases(&key, Config::parse_list(&new_aliases))
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    })
}

async fn edit_group(
    State(state): S,
    Path(name): Path<String>,
    axum::extract::Form(form): Form,
) -> Html<String> {
    let new_name = form.get("new_name").filter(|s| !s.is_empty()).cloned();
    let new_entries = form.get("new_entries").filter(|s| !s.is_empty()).cloned();

    modify_or_err(&state, |config| {
        let parsed_entries = new_entries
            .as_ref()
            .map(|raw| {
                let entries = Config::parse_list(raw);
                config
                    .validate_group_entries(&entries)
                    .map_err(|e| e.to_string())?;
                Ok::<_, String>(entries)
            })
            .transpose()?;

        let key = if let Some(ref new_name) = new_name
            && new_name != &name
        {
            config
                .rename_group(&name, new_name)
                .map_err(|e| e.to_string())?;
            new_name.clone()
        } else {
            name
        };

        if let Some(entries) = parsed_entries {
            config
                .set_group_entries(&key, entries)
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    })
}

// -- Server ------------------------------------------------------------------

pub fn router(storage: Box<dyn Storage>) -> Router {
    let state = Arc::new(AppState {
        storage: Mutex::new(storage),
    });

    Router::new()
        .route("/", get(index))
        .route("/favicon.ico", get(favicon))
        .route("/api/health", get(health))
        .route("/content", get(content))
        .route("/add/url", post(add_url))
        .route("/add/group", post(add_group))
        .route("/delete/url/{name}", post(delete_url))
        .route("/delete/group/{name}", post(delete_group))
        .route("/edit/url/{name}", post(edit_url))
        .route("/edit/group/{name}", post(edit_group))
        .with_state(state)
}

pub struct BackgroundServer {
    addr: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    handle: Option<thread::JoinHandle<anyhow::Result<()>>>,
}

impl BackgroundServer {
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn url(&self) -> String {
        format!("http://{}", self.addr)
    }
}

impl Drop for BackgroundServer {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

pub fn spawn_loopback(storage: Box<dyn Storage>) -> anyhow::Result<BackgroundServer> {
    let listener = std::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))?;
    let addr = listener.local_addr()?;
    listener.set_nonblocking(true)?;

    let app = router(storage);
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let handle = thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async move {
            let listener = tokio::net::TcpListener::from_std(listener)?;
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await?;
            Ok(())
        })
    });

    Ok(BackgroundServer {
        addr,
        shutdown: Some(shutdown_tx),
        handle: Some(handle),
    })
}

pub fn serve_addr(storage: Box<dyn Storage>, addr: SocketAddr) -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, router(storage))
            .with_graceful_shutdown(async {
                tokio::signal::ctrl_c()
                    .await
                    .expect("failed to listen for ctrl+c");
                println!("\nshutting down...");
            })
            .await?;
        Ok(())
    })
}

pub fn run_webapp(storage: Box<dyn Storage>) -> anyhow::Result<()> {
    let addr = default_webapp_addr();
    println!("bookmarks webapp: http://localhost:{}", addr.port());
    let _ = open::that(format!("http://localhost:{}", addr.port()));
    serve_addr(storage, addr)
}

pub fn wait_for_health(addr: SocketAddr) -> anyhow::Result<()> {
    for _ in 0..50 {
        if health_check(addr).is_ok_and(|body| body.contains("\"status\":\"ok\"")) {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }
    anyhow::bail!("bookmarks webapp did not become ready at {addr}")
}

fn health_check(addr: SocketAddr) -> std::io::Result<String> {
    use std::io::{Read, Write};

    let mut stream = TcpStream::connect(addr)?;
    stream.set_read_timeout(Some(Duration::from_millis(500)))?;
    stream
        .write_all(b"GET /api/health HTTP/1.1\r\nHost: bookmarks\r\nConnection: close\r\n\r\n")?;
    let mut body = String::new();
    stream.read_to_string(&mut body)?;
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    struct MemStorage {
        config: Mutex<Config>,
    }

    struct LoadErrorStorage;

    impl MemStorage {
        fn new() -> Self {
            Self {
                config: Mutex::new(Config::default()),
            }
        }
    }

    impl Storage for MemStorage {
        fn load(&self) -> anyhow::Result<Config> {
            Ok(self.config.lock().unwrap().clone())
        }

        fn save(&self, config: &Config) -> anyhow::Result<()> {
            *self.config.lock().unwrap() = config.clone();
            Ok(())
        }

        fn init(&self) -> anyhow::Result<()> {
            Ok(())
        }

        fn backend_name(&self) -> &str {
            "memory"
        }
    }

    impl Storage for LoadErrorStorage {
        fn load(&self) -> anyhow::Result<Config> {
            anyhow::bail!("bad bookmarks.toml")
        }

        fn save(&self, _config: &Config) -> anyhow::Result<()> {
            panic!("save should not be called when load fails")
        }

        fn init(&self) -> anyhow::Result<()> {
            Ok(())
        }

        fn backend_name(&self) -> &str {
            "broken"
        }
    }

    fn test_app() -> Router {
        router(Box::new(MemStorage::new()))
    }

    fn load_error_app() -> Router {
        router(Box::new(LoadErrorStorage))
    }

    async fn response_status(
        app: Router,
        method: &str,
        uri: &str,
        body: Option<&str>,
    ) -> (axum::http::StatusCode, String) {
        let req = axum::http::Request::builder().method(method).uri(uri);

        let req = if let Some(b) = body {
            req.header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(b.to_string()))
                .unwrap()
        } else {
            req.body(Body::empty()).unwrap()
        };

        let resp = app.oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let text = String::from_utf8_lossy(&bytes).to_string();
        (status, text)
    }

    #[tokio::test]
    async fn get_index_returns_200_without_external_assets() {
        let (status, body) = response_status(test_app(), "GET", "/", None).await;
        assert_eq!(status, 200);
        assert!(body.contains("Bookmarks"));
        assert!(!body.contains("https://unpkg"));
        assert!(!body.contains("script src=\"http"));
    }

    #[tokio::test]
    async fn load_errors_render_without_falling_back_to_empty_config() {
        let (status, body) = response_status(load_error_app(), "GET", "/", None).await;
        assert_eq!(status, 200);
        assert!(body.contains("error-banner"));
        assert!(body.contains("failed to load config: bad bookmarks.toml"));
    }

    #[tokio::test]
    async fn mutations_do_not_save_when_config_load_fails() {
        let (status, body) = response_status(
            load_error_app(),
            "POST",
            "/add/url",
            Some("name=rust&url=https%3A%2F%2Frust-lang.org"),
        )
        .await;
        assert_eq!(status, 200);
        assert!(body.contains("failed to load config: bad bookmarks.toml"));
    }

    #[test]
    fn js_attribute_strings_are_json_literals_and_html_escaped() {
        let literal = js_attr_string("a&b\"c'd<e>");
        assert!(literal.starts_with("&quot;"));
        assert!(literal.ends_with("&quot;"));
        assert!(literal.contains("&amp;"));
        assert!(literal.contains("\\&quot;"));
        assert!(!literal.contains('"'));
        assert!(!literal.contains('<'));
    }

    #[tokio::test]
    async fn health_returns_metadata() {
        let (status, body) = response_status(test_app(), "GET", "/api/health", None).await;
        assert_eq!(status, 200);
        assert!(body.contains(r#""status":"ok""#));
        assert!(body.contains(r#""backend":"memory""#));
    }

    #[tokio::test]
    async fn get_content_returns_200() {
        let (status, _) = response_status(test_app(), "GET", "/content", None).await;
        assert_eq!(status, 200);
    }

    #[tokio::test]
    async fn add_edit_and_delete_url() {
        let app = test_app();

        let (status, body) = response_status(
            app.clone(),
            "POST",
            "/add/url",
            Some("name=rust&url=https%3A%2F%2Frust-lang.org"),
        )
        .await;
        assert_eq!(status, 200);
        assert!(body.contains("rust"));

        let (status, body) = response_status(
            app.clone(),
            "POST",
            "/edit/url/rust",
            Some("new_name=ferris&new_aliases=rs"),
        )
        .await;
        assert_eq!(status, 200);
        assert!(body.contains("ferris"));
        assert!(body.contains("rs"));

        let (status, body) = response_status(app.clone(), "POST", "/delete/url/ferris", None).await;
        assert_eq!(status, 200);
        assert!(!body.contains("rust-lang.org"));
    }

    #[tokio::test]
    async fn add_url_empty_fields_is_noop() {
        let app = test_app();
        let (status, body) =
            response_status(app.clone(), "POST", "/add/url", Some("name=&url=")).await;
        assert_eq!(status, 200);
        assert!(body.contains("no urls yet"));
    }

    #[tokio::test]
    async fn add_edit_and_delete_group() {
        let app = test_app();

        let _ = response_status(
            app.clone(),
            "POST",
            "/add/url",
            Some("name=gh&url=https%3A%2F%2Fgithub.com"),
        )
        .await;
        let _ = response_status(
            app.clone(),
            "POST",
            "/add/url",
            Some("name=rs&url=https%3A%2F%2Frust-lang.org"),
        )
        .await;

        let (status, body) = response_status(
            app.clone(),
            "POST",
            "/add/group",
            Some("name=dev&entries=gh"),
        )
        .await;
        assert_eq!(status, 200);
        assert!(body.contains("dev"));

        let (status, body) = response_status(
            app.clone(),
            "POST",
            "/edit/group/dev",
            Some("new_name=code&new_entries=gh%2Crs"),
        )
        .await;
        assert_eq!(status, 200);
        assert!(body.contains("code"));
        assert!(body.contains("rs"));

        let (status, body) = response_status(app.clone(), "POST", "/delete/group/code", None).await;
        assert_eq!(status, 200);
        assert!(!body.contains(">code<"));
    }

    #[tokio::test]
    async fn add_group_with_missing_entries_shows_error() {
        let app = test_app();

        let (status, body) = response_status(
            app.clone(),
            "POST",
            "/add/group",
            Some("name=bad&entries=nonexistent"),
        )
        .await;
        assert_eq!(status, 200);
        assert!(body.contains("error-banner"));
    }

    #[tokio::test]
    async fn delete_nonexistent_url_shows_error() {
        let app = test_app();
        let (status, body) = response_status(app.clone(), "POST", "/delete/url/nope", None).await;
        assert_eq!(status, 200);
        assert!(body.contains("error-banner"));
    }

    #[tokio::test]
    async fn sort_by_url() {
        let app = test_app();
        let (status, _) = response_status(app.clone(), "GET", "/content?sort=url", None).await;
        assert_eq!(status, 200);
    }
}
