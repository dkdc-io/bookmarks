//! Embedded HTMX webapp for bookmarks

use axum::Router;
use axum::extract::{Path, Query, State};
use axum::response::Html;
use axum::routing::{get, post};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use bookmarks_core::config::{Config, UrlEntry};
use bookmarks_core::storage::Storage;
use bookmarks_core::strings;

struct AppState {
    storage: Mutex<Box<dyn Storage>>,
}

impl AppState {
    fn lock_storage(&self) -> std::sync::MutexGuard<'_, Box<dyn Storage>> {
        self.storage.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn load_config(&self) -> Config {
        self.lock_storage().load().unwrap_or_default()
    }

    fn save_config(&self, config: &Config) -> Result<(), String> {
        self.lock_storage().save(config).map_err(|e| e.to_string())
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn escape_js(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('"', "&quot;")
        .replace('<', "\\x3c")
        .replace('>', "\\x3e")
}

// -- HTML rendering ----------------------------------------------------------

fn page(body: &str) -> String {
    let project_url = strings::PROJECT_URL;
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>bookmarks</title>
  <script src="https://unpkg.com/htmx.org@2.0.4"></script>
  <style>
    * {{ margin: 0; padding: 0; box-sizing: border-box; }}
    html {{ background: #1a1a29; }}
    body {{ font-family: system-ui, -apple-system, sans-serif; background: #1a1a29; color: #8c8ca6; width: 640px; margin: 0 auto; padding: 32px 0; }}
    h1 {{ font-size: 1.4rem; color: #8c8ca6; margin-bottom: 8px; font-weight: 500; }}
    .subtitle {{ font-size: 0.85rem; color: #8c8ca6; margin-bottom: 24px; }}
    .subtitle a {{ color: #bf4dff; text-decoration: none; }}
    .subtitle a:hover {{ text-decoration: underline; }}
    h2 {{ font-size: 1rem; color: #8c8ca6; margin-bottom: 12px; text-transform: lowercase; }}
    .section {{ margin-bottom: 28px; }}
    table {{ width: 100%; border-collapse: collapse; table-layout: fixed; }}
    col.col-check {{ width: 28px; }}
    col.col-name {{ width: 130px; }}
    col.col-value {{ }}
    col.col-actions {{ width: 70px; }}
    th {{ text-align: left; font-size: 0.75rem; color: #666680; text-transform: uppercase; letter-spacing: 0.05em; padding: 6px 8px; border-bottom: 1px solid #2e2e47; }}
    th.sortable {{ cursor: pointer; user-select: none; }}
    th.sortable:hover {{ color: #8c8ca6; }}
    th.active {{ color: #bf4dff; }}
    td {{ padding: 6px 8px; border-bottom: 1px solid #242438; font-size: 0.85rem; vertical-align: top; overflow: hidden; text-overflow: ellipsis; }}
    td.check {{ text-align: center; overflow: visible; }}
    td.check input {{ cursor: pointer; accent-color: #bf4dff; }}
    th.check {{ text-align: center; overflow: visible; }}
    th.check input {{ cursor: pointer; accent-color: #bf4dff; }}
    td.name {{ color: #bf4dff; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }}
    td.name a {{ color: #bf4dff; text-decoration: none; }}
    td.name a:hover {{ text-decoration: underline; }}
    td.url a {{ color: #22d3ee; text-decoration: none; word-break: break-all; }}
    td.url a:hover {{ text-decoration: underline; color: #67e8f9; }}
    td.aliases {{ color: #a640f2; font-size: 0.8rem; }}
    td.entries a {{ color: #a640f2; text-decoration: none; }}
    td.entries a:hover {{ text-decoration: underline; color: #bf4dff; }}
    .actions {{ text-align: right; white-space: nowrap; }}
    .btn {{ background: none; border: 1px solid #2e2e47; color: #8c8ca6; padding: 2px 8px; border-radius: 4px; cursor: pointer; font-size: 0.75rem; }}
    .btn:hover {{ border-color: #666680; color: #edeedf; }}
    .btn-danger {{ border-color: #5c2a2a; color: #ff7373; }}
    .btn-danger:hover {{ border-color: #ff7373; color: #ffa0a0; }}
    .btn-add {{ background: #242438; border-color: #2e2e47; color: #bf4dff; white-space: nowrap; width: 72px; text-align: center; flex-shrink: 0; }}
    .btn-add:hover {{ background: #2e2e47; border-color: #666680; }}
    .bulk-bar {{ display: none; align-items: center; gap: 8px; margin-bottom: 12px; padding: 8px 12px; background: #242438; border: 1px solid #2e2e47; border-radius: 6px; }}
    .bulk-bar.visible {{ display: flex; }}
    .bulk-bar .bulk-count {{ font-size: 0.8rem; color: #bf4dff; }}
    .bulk-bar .btn {{ font-size: 0.75rem; }}
    form.inline {{ display: flex; gap: 6px; align-items: center; margin-top: 6px; }}
    form.inline input {{ background: #242438; border: 1px solid #2e2e47; color: #edeedf; padding: 5px 8px; border-radius: 4px; font-size: 0.8rem; min-width: 0; }}
    form.inline input:first-of-type {{ flex: 2; }}
    form.inline input:nth-of-type(2) {{ flex: 3; }}
    form.inline input::placeholder {{ color: #666680; }}
    form.inline input:focus {{ outline: none; border-color: #bf4dff; }}
    .copy-btn {{ background: none; border: none; color: #666680; cursor: pointer; padding: 0; line-height: 1; flex-shrink: 0; vertical-align: middle; }}
    .copy-btn:hover {{ color: #8c8ca6; }}
    .copy-btn.copied {{ color: #4ade80; }}
    td.url {{ }}
    td.url .url-cell {{ display: flex; align-items: center; gap: 6px; }}
    .error-banner {{ background: #3a1a2a; border: 1px solid #5c2a2a; color: #ff7373; padding: 8px 12px; border-radius: 6px; margin-bottom: 12px; font-size: 0.8rem; cursor: pointer; }}
    .editable {{ cursor: pointer; }}
    .editable:hover {{ background: #2e2e47; border-radius: 3px; }}
    .edit-input {{ background: #242438; border: 1px solid #bf4dff; color: #edeedf; padding: 3px 6px; border-radius: 3px; font-size: 0.8rem; width: 100%; font-family: inherit; }}
    .edit-input:focus {{ outline: none; }}
    .empty {{ color: #666680; font-style: italic; font-size: 0.85rem; padding: 12px 0; }}
    .toolbar {{ display: flex; gap: 8px; align-items: center; margin-bottom: 16px; }}
    .toolbar input {{ background: #242438; border: 1px solid #2e2e47; color: #edeedf; padding: 5px 8px; border-radius: 4px; font-size: 0.8rem; width: 200px; }}
    .toolbar input::placeholder {{ color: #666680; }}
    .toolbar input:focus {{ outline: none; border-color: #bf4dff; }}
    .tabs {{ display: flex; gap: 4px; flex-shrink: 0; }}
    .tab {{ background: none; border: 1px solid #2e2e47; color: #8c8ca6; padding: 4px 10px; border-radius: 4px; cursor: pointer; font-size: 0.75rem; }}
    .tab:hover {{ color: #edeedf; border-color: #666680; }}
    .tab.active {{ color: #bf4dff; border-color: #bf4dff; background: #382952; }}
    .counts {{ font-size: 0.7rem; color: #666680; margin-left: 3px; }}
    /* confirm modal */
    .modal-overlay {{ display: none; position: fixed; inset: 0; background: rgba(0,0,0,0.7); z-index: 100; align-items: center; justify-content: center; }}
    .modal-overlay.visible {{ display: flex; }}
    .modal {{ background: #141421; border: 1px solid #2e2e47; border-radius: 8px; padding: 24px; max-width: 400px; width: 90%; }}
    .modal h3 {{ color: #edeedf; font-size: 1rem; margin-bottom: 8px; }}
    .modal p {{ color: #8c8ca6; font-size: 0.85rem; margin-bottom: 16px; line-height: 1.4; }}
    .modal .modal-actions {{ display: flex; gap: 8px; justify-content: flex-end; }}
    .modal .btn-cancel {{ border-color: #2e2e47; color: #8c8ca6; padding: 6px 16px; font-size: 0.8rem; }}
    .modal .btn-cancel:hover {{ border-color: #666680; color: #edeedf; }}
    .modal .btn-confirm {{ background: #3a1a2a; border-color: #ff7373; color: #ff7373; padding: 6px 16px; font-size: 0.8rem; }}
    .modal .btn-confirm:hover {{ background: #4a2030; border-color: #ffa0a0; color: #ffa0a0; }}
    @media (max-width: 680px) {{ body {{ width: auto; padding: 24px 16px; }} }}
  </style>
</head>
<body>
  <h1>Bookmarks</h1>
  <p class="subtitle"><a href="{project_url}" target="_blank" rel="noopener">bookmarks</a> in your filesystem</p>
  <div id="content">
    {body}
  </div>

  <!-- confirm modal -->
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

  <script>
    // -- confirm modal ---
    var pendingAction = null;
    function confirmDelete(title, message, action) {{
      document.getElementById('confirm-title').textContent = title;
      document.getElementById('confirm-message').textContent = message;
      document.getElementById('confirm-modal').classList.add('visible');
      pendingAction = action;
      // wire up confirm button
      var btn = document.getElementById('confirm-btn');
      btn.onclick = function() {{
        var action = pendingAction;
        closeModal();
        if (action) action();
      }};
    }}
    function closeModal() {{
      document.getElementById('confirm-modal').classList.remove('visible');
      pendingAction = null;
    }}
    // close on escape or clicking overlay
    document.getElementById('confirm-modal').addEventListener('click', function(e) {{
      if (e.target === this) closeModal();
    }});
    document.addEventListener('keydown', function(e) {{
      if (e.key === 'Escape') closeModal();
    }});

    // -- open all group URLs ---
    function openGroup(urls) {{
      urls.forEach(function(u) {{ window.open(u, '_blank', 'noopener'); }});
    }}

    // -- copy to clipboard ---
    function copyUrl(btn, text) {{
      navigator.clipboard.writeText(text).then(function() {{
        btn.classList.add('copied');
        btn.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>';
        setTimeout(function() {{
          btn.classList.remove('copied');
          btn.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>';
        }}, 1500);
      }});
    }}

    // -- inline edit ---
    function startEdit(type, name, field, currentValue) {{
      var cell = event.target.closest('td');
      if (cell.querySelector('.edit-input')) return; // already editing
      var original = cell.innerHTML;
      var done = false;
      var input = document.createElement('input');
      input.className = 'edit-input';
      input.value = currentValue;
      function finish(save) {{
        if (done) return;
        done = true;
        if (save && input.value.trim() && input.value !== currentValue) {{
          submitEdit(type, name, field, input.value.trim(), cell, original);
        }} else {{
          cell.innerHTML = original;
        }}
      }}
      input.addEventListener('keydown', function(e) {{
        if (e.key === 'Enter') {{ e.preventDefault(); finish(true); }}
        if (e.key === 'Escape') {{ finish(false); }}
      }});
      input.addEventListener('blur', function() {{ finish(true); }});
      cell.innerHTML = '';
      cell.appendChild(input);
      input.focus();
      input.select();
    }}
    function submitEdit(type, name, field, value, cell, original) {{
      var params = new URLSearchParams();
      if (field === 'name') params.append('new_name', value);
      if (field === 'url') params.append('new_url', value);
      if (field === 'aliases') params.append('new_aliases', value);
      if (field === 'entries') params.append('new_entries', value);
      fetch('/edit/' + type + '/' + encodeURIComponent(name), {{method: 'POST', headers: {{'Content-Type': 'application/x-www-form-urlencoded'}}, body: params.toString()}})
        .then(function(r) {{ if (!r.ok) throw new Error(r.statusText); return r.text(); }})
        .then(function(html) {{ document.getElementById('content').innerHTML = html; }})
        .catch(function() {{ cell.innerHTML = original; }});
    }}

    // -- single delete via modal ---
    function deleteSingle(type, name) {{
      confirmDelete(
        'delete ' + type,
        'are you sure you want to delete ' + type + ' "' + name + '"? this cannot be undone.',
        function() {{
          htmx.ajax('POST', '/delete/' + type + '/' + encodeURIComponent(name), {{target: '#content', swap: 'innerHTML'}});
        }}
      );
    }}

    // -- checkbox selection ---
    function updateBulkBar() {{
      var checked = document.querySelectorAll('input.row-check:checked');
      var bar = document.getElementById('bulk-bar');
      var count = document.getElementById('bulk-count');
      if (checked.length > 0) {{
        bar.classList.add('visible');
        count.textContent = checked.length + ' selected';
      }} else {{
        bar.classList.remove('visible');
      }}
    }}
    function toggleAll(src) {{
      var boxes = document.querySelectorAll('input.row-check');
      // only toggle visible rows
      boxes.forEach(function(cb) {{
        if (cb.closest('tr').style.display !== 'none') cb.checked = src.checked;
      }});
      updateBulkBar();
    }}
    function deleteSelected() {{
      var checked = document.querySelectorAll('input.row-check:checked');
      if (checked.length === 0) return;
      var items = [];
      checked.forEach(function(cb) {{ items.push(cb.dataset.type + ' "' + cb.dataset.name + '"'); }});
      var count = checked.length;
      confirmDelete(
        'delete ' + count + ' item' + (count > 1 ? 's' : ''),
        'are you sure you want to delete: ' + items.join(', ') + '? this cannot be undone.',
        function() {{
          // collect names grouped by type
          var toDelete = [];
          checked.forEach(function(cb) {{
            toDelete.push({{type: cb.dataset.type, name: cb.dataset.name}});
          }});
          // delete sequentially, refresh at end
          var i = 0;
          function next() {{
            if (i >= toDelete.length) {{
              htmx.ajax('GET', '/content', {{target: '#content', swap: 'innerHTML'}});
              return;
            }}
            var item = toDelete[i++];
            fetch('/delete/' + item.type + '/' + encodeURIComponent(item.name), {{method: 'POST'}}).then(next);
          }}
          next();
        }}
      );
    }}

    // -- filter ---
    function filterRows() {{
      var q = document.getElementById('search').value.toLowerCase();
      document.querySelectorAll('table tr[data-filter]').forEach(function(row) {{
        row.style.display = row.getAttribute('data-filter').toLowerCase().includes(q) ? '' : 'none';
      }});
    }}
    function showTab(tab) {{
      ['urls','groups'].forEach(function(t) {{
        var el = document.getElementById('section-' + t);
        var btn = document.getElementById('tab-' + t);
        if (el) el.style.display = (t === tab || tab === 'all') ? '' : 'none';
        if (btn) btn.classList.toggle('active', t === tab);
      }});
      var allBtn = document.getElementById('tab-all');
      if (allBtn) allBtn.classList.toggle('active', tab === 'all');
      filterRows();
    }}

    // re-bind checkboxes after htmx swaps
    document.body.addEventListener('htmx:afterSwap', function() {{
      updateBulkBar();
      // uncheck select-all headers
      document.querySelectorAll('.select-all').forEach(function(cb) {{ cb.checked = false; }});
    }});
  </script>
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
    let uj = escape_js(url);
    format!(
        r##"<button class="copy-btn" onclick="copyUrl(this,'{uj}')" title="copy to clipboard"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg></button>"##
    )
}

fn url_row(name: &str, entry: &UrlEntry) -> String {
    let n = escape(name);
    let nj = escape_js(name);
    let url = entry.url();
    let u = escape(url);
    let uj = escape_js(url);
    let name_link = linked_name(name, url);
    let copy = copy_btn(url);
    let aliases = entry.aliases();
    let aliases_raw = aliases.join(", ");
    let aliases_raw_js = escape_js(&aliases_raw);
    let aliases_html = if aliases.is_empty() {
        r#"<span class="editable" style="color:#666680;font-size:0.8rem;font-style:italic">+ aliases</span>"#.to_string()
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
  <td class="name editable" ondblclick="startEdit('url','{nj}','name','{nj}')">{name_link}</td>
  <td class="url editable" ondblclick="startEdit('url','{nj}','url','{uj}')"><span class="url-cell">{copy}<a href="{u}" target="_blank" rel="noopener">{u}</a></span></td>
  <td class="aliases editable" ondblclick="startEdit('url','{nj}','aliases','{aliases_raw_js}')">{aliases_html}</td>
  <td class="actions">
    <button class="btn btn-danger" onclick="deleteSingle('url','{nj}')">delete</button>
  </td>
</tr>"##,
        aliases_filter = escape(&aliases.join(" ")),
    )
}

fn group_row(name: &str, entries: &[String], config: &Config) -> String {
    let n = escape(name);
    let nj = escape_js(name);
    // Collect resolved URLs for the "open all" action
    let urls: Vec<String> = entries
        .iter()
        .filter_map(|entry| resolve_url(entry, config).map(escape_js))
        .collect();
    let urls_arr: String = urls
        .iter()
        .map(|u| format!("'{u}'"))
        .collect::<Vec<_>>()
        .join(",");

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
    let filter_str: String = entries
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
    let entries_raw_js = escape_js(&entries.join(", "));
    format!(
        r##"<tr data-filter="{n} {filter_str}">
  <td class="check"><input type="checkbox" class="row-check" data-type="group" data-name="{n}" onchange="updateBulkBar()"></td>
  <td class="name editable" ondblclick="startEdit('group','{nj}','name','{nj}')">{name_cell}</td>
  <td class="entries editable" ondblclick="startEdit('group','{nj}','entries','{entries_raw_js}')">{entries_html}</td>
  <td class="actions">
    <button class="btn btn-danger" onclick="deleteSingle('group','{nj}')">delete</button>
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

    // Toolbar: search + tab filter
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

    // Error banner
    if let Some(msg) = error {
        let m = escape(msg);
        html.push_str(&format!(
            r##"<div class="error-banner" onclick="this.remove()">{m} <span style="margin-left:8px;cursor:pointer;opacity:0.6">✕</span></div>"##
        ));
    }

    // Bulk action bar (hidden until selection)
    html.push_str(
        r##"<div class="bulk-bar" id="bulk-bar">
  <span class="bulk-count" id="bulk-count">0 selected</span>
  <button class="btn btn-danger" onclick="deleteSelected()">delete selected</button>
  <button class="btn" onclick="document.querySelectorAll('input.row-check').forEach(function(c){{c.checked=false}});updateBulkBar()">clear</button>
</div>"##,
    );

    // Add forms at the top
    html.push_str(&format!(
        r##"<div class="section">
<form class="inline" hx-post="/add/url" hx-target="#content">
  <input name="name" placeholder="{ph_url_name}" required>
  <input name="url" placeholder="{ph_url}" required>
  <button class="btn btn-add" type="submit">+ url</button>
</form>
<form class="inline" hx-post="/add/group" hx-target="#content">
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

    // Urls section
    html.push_str(r##"<div class="section" id="section-urls"><h2>urls</h2>"##);
    if urls.is_empty() {
        html.push_str(r#"<p class="empty">no urls yet</p>"#);
    } else {
        html.push_str(&format!(
            r##"<table><colgroup><col class="col-check"><col class="col-name"><col class="col-value"><col style="width:120px"><col class="col-actions"></colgroup><tr><th class="check"><input type="checkbox" class="select-all" onchange="toggleAll(this)"></th><th class="sortable{name_cls}" hx-get="/content?sort=name" hx-target="#content">name</th><th class="sortable{url_cls}" hx-get="/content?sort=url" hx-target="#content">url</th><th>aliases</th><th></th></tr>"##,
        ));
        for (name, entry) in &urls {
            html.push_str(&url_row(name, entry));
        }
        html.push_str("</table>");
    }
    html.push_str("</div>");

    // Groups section
    html.push_str(r##"<div class="section" id="section-groups"><h2>groups</h2>"##);
    if groups.is_empty() {
        html.push_str(r#"<p class="empty">no groups yet</p>"#);
    } else {
        html.push_str(&format!(
            r##"<table><colgroup><col class="col-check"><col class="col-name"><col class="col-value"><col class="col-actions"></colgroup><tr><th class="check"><input type="checkbox" class="select-all" onchange="toggleAll(this)"></th><th class="sortable{name_cls}" hx-get="/content?sort=name" hx-target="#content">group</th><th>entries</th><th></th></tr>"##,
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

fn parse_sort(q: &ContentQuery) -> SortField {
    match q.sort.as_deref() {
        Some("url") => SortField::Url,
        _ => SortField::Name,
    }
}

async fn index(State(state): S, q: Query<ContentQuery>) -> Html<String> {
    Html(page(&render_content(
        &state.load_config(),
        parse_sort(&q),
        None,
    )))
}

async fn content(State(state): S, q: Query<ContentQuery>) -> Html<String> {
    Html(render_content(&state.load_config(), parse_sort(&q), None))
}

fn content_ok(state: &Arc<AppState>) -> Html<String> {
    Html(render_content(&state.load_config(), SortField::Name, None))
}

fn content_err(state: &Arc<AppState>, msg: &str) -> Html<String> {
    Html(render_content(
        &state.load_config(),
        SortField::Name,
        Some(msg),
    ))
}

fn save_or_err(state: &Arc<AppState>, config: &Config) -> Html<String> {
    match state.save_config(config) {
        Ok(()) => content_ok(state),
        Err(e) => content_err(state, &format!("failed to save: {e}")),
    }
}

async fn add_url(State(state): S, axum::extract::Form(form): Form) -> Html<String> {
    let name = form.get("name").cloned().unwrap_or_default();
    let url = form.get("url").cloned().unwrap_or_default();
    if !name.is_empty() && !url.is_empty() {
        let mut config = state.load_config();
        config.urls.insert(name, UrlEntry::Simple(url));
        return save_or_err(&state, &config);
    }
    content_ok(&state)
}

async fn add_group(State(state): S, axum::extract::Form(form): Form) -> Html<String> {
    let name = form.get("name").cloned().unwrap_or_default();
    let entries_raw = form.get("entries").cloned().unwrap_or_default();
    if !name.is_empty() && !entries_raw.is_empty() {
        let entries: Vec<String> = entries_raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !entries.is_empty() {
            let config = state.load_config();
            let missing: Vec<&str> = entries
                .iter()
                .filter(|e| !config.contains(e))
                .map(String::as_str)
                .collect();
            if !missing.is_empty() {
                return content_err(&state, &strings::err_group_entries_missing(&missing));
            }
            let mut config = config;
            config.groups.insert(name, entries);
            return save_or_err(&state, &config);
        }
    }
    content_ok(&state)
}

async fn delete_url(State(state): S, Path(name): Path<String>) -> Html<String> {
    let mut config = state.load_config();
    if let Err(e) = config.delete_url(&name) {
        return content_err(&state, &e.to_string());
    }
    save_or_err(&state, &config)
}

async fn delete_group(State(state): S, Path(name): Path<String>) -> Html<String> {
    let mut config = state.load_config();
    if let Err(e) = config.delete_group(&name) {
        return content_err(&state, &e.to_string());
    }
    save_or_err(&state, &config)
}

// -- Edit handlers -----------------------------------------------------------

async fn edit_url(
    State(state): S,
    Path(name): Path<String>,
    axum::extract::Form(form): Form,
) -> Html<String> {
    let mut config = state.load_config();
    let new_name = form.get("new_name").filter(|s| !s.is_empty());
    let new_url = form.get("new_url").filter(|s| !s.is_empty());

    // Rename first (can fail), then update value on the (possibly new) key
    let key = if let Some(new_name) = new_name
        && new_name != &name
    {
        if let Err(e) = config.rename_url(&name, new_name) {
            return content_err(&state, &e.to_string());
        }
        new_name.clone()
    } else {
        name
    };

    if let Some(new_url) = new_url
        && let Some(entry) = config.urls.get_mut(&key)
    {
        entry.set_url(new_url.clone());
    }

    // Update aliases if provided
    if let Some(new_aliases) = form.get("new_aliases") {
        let aliases: Vec<String> = new_aliases
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if let Some(entry) = config.urls.get_mut(&key) {
            match entry {
                UrlEntry::Simple(url) => {
                    if !aliases.is_empty() {
                        *entry = UrlEntry::Full {
                            url: url.clone(),
                            aliases,
                        };
                    }
                }
                UrlEntry::Full {
                    aliases: existing, ..
                } => {
                    *existing = aliases;
                }
            }
        }
    }

    save_or_err(&state, &config)
}

async fn edit_group(
    State(state): S,
    Path(name): Path<String>,
    axum::extract::Form(form): Form,
) -> Html<String> {
    let mut config = state.load_config();
    let new_name = form.get("new_name").filter(|s| !s.is_empty());
    let new_entries = form.get("new_entries").filter(|s| !s.is_empty());

    // Parse and validate entries before any mutation
    let parsed_entries = if let Some(new_entries) = new_entries {
        let entries: Vec<String> = new_entries
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let missing: Vec<&str> = entries
            .iter()
            .filter(|e| !config.contains(e))
            .map(String::as_str)
            .collect();
        if !missing.is_empty() {
            return content_err(&state, &strings::err_group_entries_missing(&missing));
        }
        Some(entries)
    } else {
        None
    };

    // Rename first, then update entries on the (possibly new) key
    let key = if let Some(new_name) = new_name
        && new_name != &name
    {
        if let Err(e) = config.rename_group(&name, new_name) {
            return content_err(&state, &e.to_string());
        }
        new_name.clone()
    } else {
        name
    };

    if let Some(entries) = parsed_entries
        && let Some(existing) = config.groups.get_mut(&key)
    {
        *existing = entries;
    }

    save_or_err(&state, &config)
}

// -- Server ------------------------------------------------------------------

fn create_router(storage: Box<dyn Storage>) -> Router {
    let state = Arc::new(AppState {
        storage: Mutex::new(storage),
    });

    Router::new()
        .route("/", get(index))
        .route("/content", get(content))
        .route("/add/url", post(add_url))
        .route("/add/group", post(add_group))
        .route("/delete/url/{name}", post(delete_url))
        .route("/delete/group/{name}", post(delete_group))
        .route("/edit/url/{name}", post(edit_url))
        .route("/edit/group/{name}", post(edit_group))
        .with_state(state)
}

pub fn run_webapp(storage: Box<dyn Storage>) -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let port: u16 = 1414;
        let app = create_router(storage);
        let addr = SocketAddr::from(([127, 0, 0, 1], port));

        println!("bookmarks webapp: http://localhost:{port}");
        let _ = open::that(format!("http://localhost:{port}"));

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app)
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
