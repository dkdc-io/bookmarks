//! Desktop app for bookmarks

use iced::widget::{
    Column, button, center, checkbox, column, container, mouse_area, row, scrollable, text,
    text_input,
};
use iced::{Element, Length, Size, Theme};
use std::collections::HashSet;

use bookmarks_core::config::{Config, UrlEntry};
use bookmarks_core::storage::Storage;
use bookmarks_core::strings;

// -- Colors ------------------------------------------------------------------

mod colors {
    use iced::Color;

    pub const BG_DARK: Color = Color::from_rgb(0.10, 0.10, 0.16);
    pub const BG_INPUT: Color = Color::from_rgb(0.14, 0.14, 0.22);
    pub const BG_HOVER: Color = Color::from_rgb(0.18, 0.18, 0.28);
    pub const BORDER: Color = Color::from_rgb(0.18, 0.18, 0.28);
    pub const BORDER_FOCUS: Color = Color::from_rgb(0.75, 0.30, 1.0);
    pub const PURPLE: Color = Color::from_rgb(0.75, 0.30, 1.0);
    pub const PURPLE_DIM: Color = Color::from_rgb(0.65, 0.25, 0.95);
    pub const CYAN: Color = Color::from_rgb(0.13, 0.83, 0.93);
    pub const TEXT: Color = Color::from_rgb(0.55, 0.55, 0.65);
    pub const TEXT_BRIGHT: Color = Color::from_rgb(0.93, 0.93, 0.87);
    pub const TEXT_DIM: Color = Color::from_rgb(0.40, 0.40, 0.50);
    pub const RED: Color = Color::from_rgb(1.0, 0.45, 0.45);
    pub const RED_BG: Color = Color::from_rgb(0.23, 0.10, 0.17);
    pub const RED_BORDER: Color = Color::from_rgb(0.36, 0.17, 0.17);
    pub const TAB_ACTIVE_BG: Color = Color::from_rgb(0.22, 0.16, 0.32);
}

// -- Types -------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    All,
    Urls,
    Groups,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortField {
    Name,
    Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ItemKind {
    Url,
    Group,
}

/// Row-level edit state: both name and value are editable at once.
#[derive(Debug, Clone)]
struct RowEditState {
    kind: ItemKind,
    original_name: String,
    edit_name: String,
    edit_value: String,
    /// Comma-separated aliases (only used for Url items)
    edit_aliases: String,
}

#[derive(Debug, Clone)]
struct ConfirmState {
    title: String,
    message: String,
    action: ConfirmAction,
}

#[derive(Debug, Clone)]
enum ConfirmAction {
    DeleteSingle(ItemKind, String),
    DeleteBulk(Vec<(ItemKind, String)>),
}

/// Right-click context menu state
#[derive(Debug, Clone)]
struct ContextMenuState {
    kind: ItemKind,
    name: String,
    /// Resolved URLs for this item
    urls: Vec<String>,
}

// -- Messages ----------------------------------------------------------------

#[derive(Debug, Clone)]
enum Message {
    TabSelected(Tab),
    SearchChanged(String),
    SortBy(SortField),

    AddUrlName(String),
    AddUrlValue(String),
    SubmitUrl,
    AddGroupName(String),
    AddGroupEntries(String),
    SubmitGroup,

    ToggleSelect(ItemKind, String),
    ToggleSelectAll,
    ClearSelection,
    DeleteSelected,

    RequestDelete(ItemKind, String),
    /// Enter row edit mode: (kind, name, current_name, current_value, current_aliases)
    StartRowEdit(ItemKind, String, String, String, String),
    EditNameChanged(String),
    EditValueChanged(String),
    EditAliasesChanged(String),
    SaveEdit,
    CancelEdit,

    OpenUrl(String),
    OpenUrls(Vec<String>),
    CopyUrl(String),

    /// Show right-click context menu for a row
    ShowContextMenu(ItemKind, String, Vec<String>),

    ConfirmYes,
    ConfirmNo,

    DismissError,
}

// -- App State ---------------------------------------------------------------

struct Bookmarks {
    storage: Box<dyn Storage>,
    config: Config,

    tab: Tab,
    search: String,
    sort: SortField,

    add_url_name: String,
    add_url_value: String,
    add_group_name: String,
    add_group_entries: String,

    selected: HashSet<(ItemKind, String)>,
    editing: Option<RowEditState>,
    context_menu: Option<ContextMenuState>,
    confirm: Option<ConfirmState>,
    error: Option<String>,
}

impl Bookmarks {
    fn new(storage: Box<dyn Storage>) -> (Self, iced::Task<Message>) {
        let config = storage.load().unwrap_or_default();
        (
            Self {
                storage,
                config,
                tab: Tab::All,
                search: String::new(),
                sort: SortField::Name,
                add_url_name: String::new(),
                add_url_value: String::new(),
                add_group_name: String::new(),
                add_group_entries: String::new(),
                selected: HashSet::new(),
                editing: None,
                context_menu: None,
                confirm: None,
                error: None,
            },
            iced::Task::none(),
        )
    }

    fn save(&mut self) {
        if let Err(e) = self.storage.save(&self.config) {
            self.error = Some(format!("failed to save: {e}"));
        }
    }

    /// Save the current row edit and clear edit state.
    fn save_row_edit(&mut self) {
        if let Some(edit) = self.editing.take() {
            let name = edit.edit_name.trim().to_string();
            let value = edit.edit_value.trim().to_string();
            if name.is_empty() || value.is_empty() {
                return;
            }
            // Apply value change first, then aliases, then name (rename cascades).
            // If an error occurs, restore edit state so the user can retry.
            self.apply_edit(edit.kind, &edit.original_name, "value", &value);
            if self.error.is_some() {
                self.editing = Some(edit);
                return;
            }
            if edit.kind == ItemKind::Url {
                self.apply_edit(
                    edit.kind,
                    &edit.original_name,
                    "aliases",
                    &edit.edit_aliases,
                );
                if self.error.is_some() {
                    self.editing = Some(RowEditState {
                        kind: edit.kind,
                        original_name: edit.original_name,
                        edit_name: name,
                        edit_value: value,
                        edit_aliases: edit.edit_aliases,
                    });
                    return;
                }
            }
            if name != edit.original_name {
                self.apply_edit(edit.kind, &edit.original_name, "name", &name);
                if self.error.is_some() {
                    self.editing = Some(RowEditState {
                        kind: edit.kind,
                        original_name: edit.original_name,
                        edit_name: name,
                        edit_value: value,
                        edit_aliases: edit.edit_aliases,
                    });
                }
            }
        }
    }

    fn resolve_url<'a>(&'a self, name: &str) -> Option<&'a str> {
        bookmarks_core::open::resolve_uri(name, &self.config).ok()
    }

    fn matches_filter(&self, haystack: &str) -> bool {
        if self.search.is_empty() {
            return true;
        }
        let q = self.search.to_lowercase();
        haystack.to_lowercase().contains(&q)
    }

    fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::TabSelected(tab) => {
                self.save_row_edit();
                self.context_menu = None;
                self.tab = tab;
            }
            Message::SearchChanged(s) => {
                self.save_row_edit();
                self.context_menu = None;
                self.search = s;
            }
            Message::SortBy(field) => {
                self.sort = if self.sort == field {
                    if field == SortField::Name {
                        SortField::Value
                    } else {
                        SortField::Name
                    }
                } else {
                    field
                };
            }

            Message::AddUrlName(s) => self.add_url_name = s,
            Message::AddUrlValue(s) => self.add_url_value = s,
            Message::SubmitUrl => {
                let name = self.add_url_name.trim().to_string();
                let url = self.add_url_value.trim().to_string();
                if !name.is_empty() && !url.is_empty() {
                    self.config.urls.insert(name, UrlEntry::Simple(url));
                    self.save();
                    self.add_url_name.clear();
                    self.add_url_value.clear();
                }
            }
            Message::AddGroupName(s) => self.add_group_name = s,
            Message::AddGroupEntries(s) => self.add_group_entries = s,
            Message::SubmitGroup => {
                let name = self.add_group_name.trim().to_string();
                let raw = self.add_group_entries.trim().to_string();
                if !name.is_empty() && !raw.is_empty() {
                    let entries: Vec<String> = raw
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    let missing: Vec<&str> = entries
                        .iter()
                        .filter(|e| !self.config.contains(e))
                        .map(String::as_str)
                        .collect();
                    if !missing.is_empty() {
                        self.error = Some(strings::err_group_entries_missing(&missing));
                    } else {
                        self.config.groups.insert(name, entries);
                        self.save();
                        self.add_group_name.clear();
                        self.add_group_entries.clear();
                        self.error = None;
                    }
                }
            }

            Message::ToggleSelect(kind, name) => {
                let key = (kind, name);
                if self.selected.contains(&key) {
                    self.selected.remove(&key);
                } else {
                    self.selected.insert(key);
                }
            }
            Message::ToggleSelectAll => {
                let visible = self.visible_items();
                let all_selected = visible.iter().all(|item| self.selected.contains(item));
                if all_selected {
                    self.selected.clear();
                } else {
                    for item in visible {
                        self.selected.insert(item);
                    }
                }
            }
            Message::ClearSelection => {
                self.selected.clear();
            }
            Message::DeleteSelected => {
                let items: Vec<(ItemKind, String)> = self.selected.iter().cloned().collect();
                if !items.is_empty() {
                    let labels: Vec<String> = items
                        .iter()
                        .map(|(k, n)| {
                            let kind_str = match k {
                                ItemKind::Url => "url",
                                ItemKind::Group => "group",
                            };
                            format!("{kind_str} \"{n}\"")
                        })
                        .collect();
                    self.confirm = Some(ConfirmState {
                        title: format!(
                            "delete {} item{}",
                            items.len(),
                            if items.len() > 1 { "s" } else { "" }
                        ),
                        message: format!(
                            "are you sure you want to delete: {}? this cannot be undone.",
                            labels.join(", ")
                        ),
                        action: ConfirmAction::DeleteBulk(items),
                    });
                }
            }

            Message::RequestDelete(kind, name) => {
                let kind_str = match kind {
                    ItemKind::Url => "url",
                    ItemKind::Group => "group",
                };
                self.confirm = Some(ConfirmState {
                    title: format!("delete {kind_str}"),
                    message: format!(
                        "are you sure you want to delete {kind_str} \"{name}\"? this cannot be undone."
                    ),
                    action: ConfirmAction::DeleteSingle(kind, name),
                });
            }
            Message::StartRowEdit(
                kind,
                original_name,
                current_name,
                current_value,
                current_aliases,
            ) => {
                self.save_row_edit();
                self.context_menu = None;
                self.editing = Some(RowEditState {
                    kind,
                    original_name,
                    edit_name: current_name,
                    edit_value: current_value,
                    edit_aliases: current_aliases,
                });
            }
            Message::EditNameChanged(s) => {
                if let Some(ref mut edit) = self.editing {
                    edit.edit_name = s;
                }
            }
            Message::EditAliasesChanged(s) => {
                if let Some(ref mut edit) = self.editing {
                    edit.edit_aliases = s;
                }
            }
            Message::EditValueChanged(s) => {
                if let Some(ref mut edit) = self.editing {
                    edit.edit_value = s;
                }
            }
            Message::SaveEdit => {
                self.save_row_edit();
            }
            Message::CancelEdit => {
                self.editing = None;
            }
            Message::OpenUrl(url) => {
                self.context_menu = None;
                let _ = open::that(&url);
            }
            Message::OpenUrls(urls) => {
                self.context_menu = None;
                for url in &urls {
                    let _ = open::that(url);
                }
            }
            Message::CopyUrl(url) => {
                self.context_menu = None;
                return iced::clipboard::write(url);
            }
            Message::ShowContextMenu(kind, name, urls) => {
                self.context_menu = Some(ContextMenuState { kind, name, urls });
            }

            Message::ConfirmYes => {
                if let Some(confirm) = self.confirm.take() {
                    match confirm.action {
                        ConfirmAction::DeleteSingle(kind, name) => {
                            self.delete_item(kind, &name);
                            self.selected.remove(&(kind, name));
                        }
                        ConfirmAction::DeleteBulk(items) => {
                            for (kind, name) in items {
                                self.delete_item(kind, &name);
                            }
                            self.selected.clear();
                        }
                    }
                    self.save();
                }
            }
            Message::ConfirmNo => {
                self.confirm = None;
            }

            Message::DismissError => {
                self.error = None;
            }
        }
        iced::Task::none()
    }

    fn delete_item(&mut self, kind: ItemKind, name: &str) {
        let result = match kind {
            ItemKind::Url => self.config.delete_url(name),
            ItemKind::Group => self.config.delete_group(name),
        };
        if let Err(e) = result {
            self.error = Some(e.to_string());
        }
    }

    fn apply_edit(&mut self, kind: ItemKind, name: &str, field: &str, value: &str) {
        match (kind, field) {
            (ItemKind::Url, "name") => {
                if value != name
                    && let Err(e) = self.config.rename_url(name, value)
                {
                    self.error = Some(e.to_string());
                    return;
                }
            }
            (ItemKind::Url, "value") => {
                if let Some(entry) = self.config.urls.get_mut(name) {
                    entry.set_url(value.to_string());
                }
            }
            (ItemKind::Url, "aliases") => {
                let new_aliases: Vec<String> = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if let Some(entry) = self.config.urls.get_mut(name) {
                    // Replace aliases entirely
                    match entry {
                        UrlEntry::Simple(url) => {
                            if !new_aliases.is_empty() {
                                *entry = UrlEntry::Full {
                                    url: url.clone(),
                                    aliases: new_aliases,
                                };
                            }
                        }
                        UrlEntry::Full { aliases, .. } => {
                            *aliases = new_aliases;
                        }
                    }
                }
            }
            (ItemKind::Group, "name") => {
                if value != name
                    && let Err(e) = self.config.rename_group(name, value)
                {
                    self.error = Some(e.to_string());
                    return;
                }
            }
            (ItemKind::Group, "value") => {
                let entries: Vec<String> = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                let missing: Vec<&str> = entries
                    .iter()
                    .filter(|e| !self.config.contains(e))
                    .map(String::as_str)
                    .collect();
                if !missing.is_empty() {
                    self.error = Some(strings::err_group_entries_missing(&missing));
                    return;
                }
                if let Some(existing) = self.config.groups.get_mut(name) {
                    *existing = entries;
                }
            }
            _ => {}
        }
        self.error = None;
        self.save();
    }

    fn visible_items(&self) -> Vec<(ItemKind, String)> {
        let mut items = Vec::new();
        if self.tab == Tab::All || self.tab == Tab::Urls {
            for (name, entry) in &self.config.urls {
                let aliases_str = entry.aliases().join(", ");
                if self.matches_filter(&format!("{name} {} {aliases_str}", entry.url())) {
                    items.push((ItemKind::Url, name.clone()));
                }
            }
        }
        if self.tab == Tab::All || self.tab == Tab::Groups {
            for (name, entries) in &self.config.groups {
                let filter_str = format!("{name} {}", entries.join(", "));
                if self.matches_filter(&filter_str) {
                    items.push((ItemKind::Group, name.clone()));
                }
            }
        }
        items
    }

    // -- View ----------------------------------------------------------------

    fn view(&self) -> Element<'_, Message> {
        let mut content = column![].spacing(16).width(Length::Fill);

        // Title
        content = content.push(
            column![
                text("Bookmarks").size(24).color(colors::TEXT),
                iced::widget::rich_text::<String, Message, _, _>([
                    iced::widget::span("bookmarks")
                        .size(13)
                        .color(colors::PURPLE)
                        .link(strings::PROJECT_URL.to_string()),
                    iced::widget::span(" in your filesystem")
                        .size(13)
                        .color(colors::TEXT_DIM),
                ])
                .on_link_click(Message::OpenUrl),
            ]
            .spacing(4),
        );

        // Toolbar
        content = content.push(self.view_toolbar());

        // Error banner
        if let Some(ref msg) = self.error {
            content = content.push(self.view_error(msg));
        }

        // Bulk bar
        if !self.selected.is_empty() {
            content = content.push(self.view_bulk_bar());
        }

        // Add forms
        content = content.push(self.view_add_forms());

        // Sections
        let mut sections = column![].spacing(20);
        if self.tab == Tab::All || self.tab == Tab::Urls {
            sections = sections.push(self.view_urls_section());
        }
        if self.tab == Tab::All || self.tab == Tab::Groups {
            sections = sections.push(self.view_groups_section());
        }
        content = content.push(sections);

        let body = scrollable(
            container(content)
                .width(640)
                .padding(32)
                .center_x(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill);

        let bg_style = |_: &_| container::Style {
            background: Some(iced::Background::Color(colors::BG_DARK)),
            ..Default::default()
        };

        if let Some(ref confirm) = self.confirm {
            let overlay = self.view_confirm_modal(confirm);
            iced::widget::stack![
                container(body)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(bg_style),
                overlay,
            ]
            .into()
        } else {
            container(body)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(bg_style)
                .into()
        }
    }

    fn view_toolbar(&self) -> Element<'_, Message> {
        let search = text_input(strings::PH_FILTER, &self.search)
            .on_input(Message::SearchChanged)
            .size(13)
            .width(200)
            .style(|_, status| input_style(status));

        let tab_btn = |label: &str, count: usize, tab: Tab| -> Element<'_, Message> {
            let is_active = self.tab == tab;
            let label_str = format!("{label} {count}");
            button(text(label_str).size(12).color(if is_active {
                colors::PURPLE
            } else {
                colors::TEXT
            }))
            .on_press(Message::TabSelected(tab))
            .padding([4, 10])
            .style(move |_, status| tab_button_style(is_active, status))
            .into()
        };

        let total = self.config.urls.len() + self.config.groups.len();

        let tabs = row![
            tab_btn("all", total, Tab::All),
            tab_btn("urls", self.config.urls.len(), Tab::Urls),
            tab_btn("groups", self.config.groups.len(), Tab::Groups),
        ]
        .spacing(4);

        row![search, iced::widget::Space::new().width(Length::Fill), tabs]
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .into()
    }

    fn view_error(&self, msg: &str) -> Element<'_, Message> {
        button(text(format!("{msg}  x")).size(13).color(colors::RED))
            .on_press(Message::DismissError)
            .padding([8, 12])
            .style(|_, _| button::Style {
                background: Some(iced::Background::Color(colors::RED_BG)),
                border: iced::Border {
                    color: colors::RED_BORDER,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                text_color: colors::RED,
                ..Default::default()
            })
            .width(Length::Fill)
            .into()
    }

    fn view_bulk_bar(&self) -> Element<'_, Message> {
        let count_text = text(format!("{} selected", self.selected.len()))
            .size(13)
            .color(colors::PURPLE);

        let delete_btn = button(text("delete selected").size(12).color(colors::RED))
            .on_press(Message::DeleteSelected)
            .padding([4, 8])
            .style(|_, _| danger_button_style());

        let clear_btn = button(text("clear").size(12).color(colors::TEXT))
            .on_press(Message::ClearSelection)
            .padding([4, 8])
            .style(|_, _| default_button_style());

        container(
            row![count_text, delete_btn, clear_btn]
                .spacing(8)
                .align_y(iced::Alignment::Center),
        )
        .padding([8, 12])
        .style(|_| container::Style {
            background: Some(iced::Background::Color(colors::BG_INPUT)),
            border: iced::Border {
                color: colors::BORDER,
                width: 1.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        })
        .width(Length::Fill)
        .into()
    }

    fn view_add_forms(&self) -> Element<'_, Message> {
        let url_form = row![
            text_input(strings::PH_URL_NAME, &self.add_url_name)
                .on_input(Message::AddUrlName)
                .on_submit(Message::SubmitUrl)
                .size(13)
                .width(Length::FillPortion(2))
                .style(|_, status| input_style(status)),
            text_input(strings::PH_URL, &self.add_url_value)
                .on_input(Message::AddUrlValue)
                .on_submit(Message::SubmitUrl)
                .size(13)
                .width(Length::FillPortion(3))
                .style(|_, status| input_style(status)),
            button(text("+ url").size(12).color(colors::PURPLE))
                .on_press(Message::SubmitUrl)
                .padding([5, 8])
                .width(72)
                .style(|_, _| add_button_style()),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center);

        let group_form = row![
            text_input(strings::PH_GROUP_NAME, &self.add_group_name)
                .on_input(Message::AddGroupName)
                .on_submit(Message::SubmitGroup)
                .size(13)
                .width(Length::FillPortion(2))
                .style(|_, status| input_style(status)),
            text_input(strings::PH_GROUP_ENTRIES, &self.add_group_entries)
                .on_input(Message::AddGroupEntries)
                .on_submit(Message::SubmitGroup)
                .size(13)
                .width(Length::FillPortion(3))
                .style(|_, status| input_style(status)),
            button(text("+ group").size(12).color(colors::PURPLE))
                .on_press(Message::SubmitGroup)
                .padding([5, 8])
                .width(72)
                .style(|_, _| add_button_style()),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center);

        column![url_form, group_form].spacing(6).into()
    }

    fn view_urls_section(&self) -> Element<'_, Message> {
        let mut urls: Vec<_> = self.config.urls.iter().collect();
        match self.sort {
            SortField::Name => urls.sort_by_key(|(k, _)| k.as_str()),
            SortField::Value => urls.sort_by_key(|(_, v)| v.url()),
        }

        let header = self.view_table_header("name", "url");

        let mut rows = Column::new().spacing(0);
        let mut visible_count = 0;
        for (name, entry) in &urls {
            let aliases_str = entry.aliases().join(", ");
            if !self.matches_filter(&format!("{name} {} {aliases_str}", entry.url())) {
                continue;
            }
            visible_count += 1;
            rows = rows.push(self.view_url_row(name, entry));
            rows = rows.push(iced::widget::rule::horizontal(1).style(|_| rule_style()));
        }

        let body: Element<'_, Message> = if visible_count == 0 {
            text("no urls yet").size(13).color(colors::TEXT_DIM).into()
        } else {
            column![
                header,
                iced::widget::rule::horizontal(1).style(|_| rule_style()),
                rows
            ]
            .into()
        };

        column![text("urls").size(16).color(colors::TEXT), body]
            .spacing(8)
            .into()
    }

    fn view_groups_section(&self) -> Element<'_, Message> {
        let mut groups: Vec<_> = self.config.groups.iter().collect();
        groups.sort_by_key(|(k, _)| k.as_str());

        let header = self.view_table_header("group", "entries");

        let mut rows = Column::new().spacing(0);
        let mut visible_count = 0;
        for (name, entries) in &groups {
            let filter_str = format!("{name} {}", entries.join(", "));
            if !self.matches_filter(&filter_str) {
                continue;
            }
            visible_count += 1;
            rows = rows.push(self.view_group_row(name, entries));
            rows = rows.push(iced::widget::rule::horizontal(1).style(|_| rule_style()));
        }

        let body: Element<'_, Message> = if visible_count == 0 {
            text("no groups yet")
                .size(13)
                .color(colors::TEXT_DIM)
                .into()
        } else {
            column![
                header,
                iced::widget::rule::horizontal(1).style(|_| rule_style()),
                rows
            ]
            .into()
        };

        column![text("groups").size(16).color(colors::TEXT), body]
            .spacing(8)
            .into()
    }

    fn view_table_header<'a>(&self, col1: &str, col2: &str) -> Element<'a, Message> {
        let name_active = self.sort == SortField::Name;
        let value_active = self.sort == SortField::Value;

        let select_all = checkbox(false)
            .on_toggle(|_| Message::ToggleSelectAll)
            .size(14)
            .style(|_, _| checkbox_style());

        let name_header = button(text(col1.to_uppercase()).size(11).color(if name_active {
            colors::PURPLE
        } else {
            colors::TEXT_DIM
        }))
        .on_press(Message::SortBy(SortField::Name))
        .padding(0)
        .style(|_, _| button::Style::default());

        let value_header = button(text(col2.to_uppercase()).size(11).color(if value_active {
            colors::PURPLE
        } else {
            colors::TEXT_DIM
        }))
        .on_press(Message::SortBy(SortField::Value))
        .padding(0)
        .style(|_, _| button::Style::default());

        row![
            container(select_all).width(28),
            container(name_header).width(130),
            container(value_header).width(Length::Fill),
            container(text("").size(11)).width(120),
        ]
        .spacing(8)
        .padding([6, 8])
        .align_y(iced::Alignment::Center)
        .into()
    }

    /// Resolve all URLs for an item (for context menu).
    fn resolve_item_urls(&self, kind: ItemKind, name: &str) -> Vec<String> {
        match kind {
            ItemKind::Url => self
                .config
                .urls
                .get(name)
                .map(|e| vec![e.url().to_string()])
                .unwrap_or_default(),
            ItemKind::Group => self
                .config
                .groups
                .get(name)
                .map(|entries| {
                    entries
                        .iter()
                        .filter_map(|e| self.resolve_url(e).map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    /// Check if context menu is showing for a given row.
    fn has_context_menu(&self, kind: ItemKind, name: &str) -> bool {
        self.context_menu
            .as_ref()
            .is_some_and(|c| c.kind == kind && c.name == name)
    }

    /// Check if a row is currently being edited.
    fn is_editing(&self, kind: ItemKind, name: &str) -> bool {
        self.editing
            .as_ref()
            .is_some_and(|e| e.kind == kind && e.original_name == name)
    }

    fn view_url_row<'a>(&'a self, name: &'a str, entry: &'a UrlEntry) -> Element<'a, Message> {
        let url = entry.url();
        let is_selected = self.selected.contains(&(ItemKind::Url, name.to_string()));
        let cb = checkbox(is_selected)
            .on_toggle({
                let name = name.to_string();
                move |_| Message::ToggleSelect(ItemKind::Url, name.clone())
            })
            .size(14)
            .style(|_, _| checkbox_style());

        if self.is_editing(ItemKind::Url, name) {
            let edit = self.editing.as_ref().unwrap();
            return self.view_url_edit_row(
                cb.into(),
                &edit.edit_name,
                &edit.edit_value,
                &edit.edit_aliases,
            );
        }

        let name_cell = button(text(name).size(13).color(colors::PURPLE))
            .on_press(Message::OpenUrl(url.to_string()))
            .padding([2, 4])
            .width(Length::Fill)
            .style(|_, status| link_cell_style(status));

        // Show URL + aliases in the value column
        let aliases = entry.aliases();
        let url_cell: Element<'_, Message> = if aliases.is_empty() {
            button(text(url).size(13).color(colors::CYAN))
                .on_press(Message::OpenUrl(url.to_string()))
                .padding([2, 4])
                .width(Length::Fill)
                .style(|_, status| link_cell_style(status))
                .into()
        } else {
            let alias_text = format!(" ({})", aliases.join(", "));
            button(
                row![
                    text(url).size(13).color(colors::CYAN),
                    text(alias_text).size(11).color(colors::TEXT_DIM),
                ]
                .spacing(4)
                .align_y(iced::Alignment::Center),
            )
            .on_press(Message::OpenUrl(url.to_string()))
            .padding([2, 4])
            .width(Length::Fill)
            .style(|_, status| link_cell_style(status))
            .into()
        };

        let aliases_str = entry.aliases().join(", ");
        let actions = self.view_row_actions_or_context(ItemKind::Url, name, url, &aliases_str);

        let r = row![
            container(cb).width(28),
            container(name_cell).width(130).clip(true),
            container(url_cell).width(Length::Fill).clip(true),
            container(actions).width(120),
        ]
        .spacing(8)
        .padding([6, 8])
        .align_y(iced::Alignment::Center);

        let urls = self.resolve_item_urls(ItemKind::Url, name);
        mouse_area(r)
            .on_right_press(Message::ShowContextMenu(
                ItemKind::Url,
                name.to_string(),
                urls,
            ))
            .into()
    }

    fn view_group_row<'a>(&'a self, name: &'a str, entries: &'a [String]) -> Element<'a, Message> {
        let is_selected = self.selected.contains(&(ItemKind::Group, name.to_string()));
        let cb = checkbox(is_selected)
            .on_toggle({
                let name = name.to_string();
                move |_| Message::ToggleSelect(ItemKind::Group, name.clone())
            })
            .size(14)
            .style(|_, _| checkbox_style());

        if self.is_editing(ItemKind::Group, name) {
            let edit = self.editing.as_ref().unwrap();
            return self.view_edit_row(
                cb.into(),
                &edit.edit_name,
                "group",
                &edit.edit_value,
                "entries",
            );
        }

        let urls: Vec<String> = entries
            .iter()
            .filter_map(|e| self.resolve_url(e).map(String::from))
            .collect();

        let name_cell: Element<'_, Message> = if !urls.is_empty() {
            button(text(name).size(13).color(colors::PURPLE))
                .on_press(Message::OpenUrls(urls))
                .padding([2, 4])
                .width(Length::Fill)
                .style(|_, status| link_cell_style(status))
                .into()
        } else {
            container(text(name).size(13).color(colors::PURPLE))
                .padding([2, 4])
                .width(Length::Fill)
                .into()
        };

        // Each entry is clickable if it resolves to a URL
        let mut entry_widgets: Vec<Element<'_, Message>> = Vec::new();
        for (i, entry) in entries.iter().enumerate() {
            if i > 0 {
                entry_widgets.push(text(", ").size(13).color(colors::TEXT_DIM).into());
            }
            let url = self.resolve_url(entry).map(String::from);
            if let Some(url) = url {
                entry_widgets.push(
                    button(text(entry.as_str()).size(13).color(colors::PURPLE_DIM))
                        .on_press(Message::OpenUrl(url))
                        .padding(0)
                        .style(|_, status| link_cell_style(status))
                        .into(),
                );
            } else {
                entry_widgets.push(
                    text(entry.as_str())
                        .size(13)
                        .color(colors::PURPLE_DIM)
                        .into(),
                );
            }
        }
        let entries_cell = container(
            row(entry_widgets)
                .spacing(0)
                .align_y(iced::Alignment::Center),
        )
        .padding([2, 4])
        .width(Length::Fill);

        let actions =
            self.view_row_actions_or_context(ItemKind::Group, name, &entries.join(", "), "");

        let r = row![
            container(cb).width(28),
            container(name_cell).width(130).clip(true),
            container(entries_cell).width(Length::Fill).clip(true),
            container(actions).width(120),
        ]
        .spacing(8)
        .padding([6, 8])
        .align_y(iced::Alignment::Center);

        let urls = self.resolve_item_urls(ItemKind::Group, name);
        mouse_area(r)
            .on_right_press(Message::ShowContextMenu(
                ItemKind::Group,
                name.to_string(),
                urls,
            ))
            .into()
    }

    /// A URL row in edit mode: name, url, aliases + save/cancel.
    fn view_url_edit_row<'a>(
        &self,
        cb: Element<'a, Message>,
        edit_name: &str,
        edit_url: &str,
        edit_aliases: &str,
    ) -> Element<'a, Message> {
        let name_input = text_input("name", edit_name)
            .on_input(Message::EditNameChanged)
            .on_submit(Message::SaveEdit)
            .size(13)
            .width(Length::Fill)
            .style(|_, status| edit_input_style(status));

        let url_input = text_input("url", edit_url)
            .on_input(Message::EditValueChanged)
            .on_submit(Message::SaveEdit)
            .size(13)
            .width(Length::Fill)
            .style(|_, status| edit_input_style(status));

        let aliases_input = text_input("aliases (comma-separated)", edit_aliases)
            .on_input(Message::EditAliasesChanged)
            .on_submit(Message::SaveEdit)
            .size(13)
            .width(Length::Fill)
            .style(|_, status| edit_input_style(status));

        let save_btn = button(text("save").size(12).color(colors::PURPLE))
            .on_press(Message::SaveEdit)
            .padding([2, 8])
            .style(|_, _| add_button_style());

        let cancel_btn = button(text("cancel").size(12).color(colors::TEXT_DIM))
            .on_press(Message::CancelEdit)
            .padding([2, 8])
            .style(|_, _| default_button_style());

        row![
            container(cb).width(28),
            container(name_input).width(100),
            container(url_input).width(Length::FillPortion(3)),
            container(aliases_input).width(Length::FillPortion(2)),
            row![save_btn, cancel_btn]
                .spacing(4)
                .align_y(iced::Alignment::Center),
        ]
        .spacing(8)
        .padding([6, 8])
        .align_y(iced::Alignment::Center)
        .into()
    }

    /// A row in edit mode: two text inputs + save/cancel buttons.
    fn view_edit_row<'a>(
        &self,
        cb: Element<'a, Message>,
        edit_name: &str,
        name_placeholder: &str,
        edit_value: &str,
        value_placeholder: &str,
    ) -> Element<'a, Message> {
        let name_input = text_input(name_placeholder, edit_name)
            .on_input(Message::EditNameChanged)
            .on_submit(Message::SaveEdit)
            .size(13)
            .width(Length::Fill)
            .style(|_, status| edit_input_style(status));

        let value_input = text_input(value_placeholder, edit_value)
            .on_input(Message::EditValueChanged)
            .on_submit(Message::SaveEdit)
            .size(13)
            .width(Length::Fill)
            .style(|_, status| edit_input_style(status));

        let save_btn = button(text("save").size(12).color(colors::PURPLE))
            .on_press(Message::SaveEdit)
            .padding([2, 8])
            .style(|_, _| add_button_style());

        let cancel_btn = button(text("cancel").size(12).color(colors::TEXT_DIM))
            .on_press(Message::CancelEdit)
            .padding([2, 8])
            .style(|_, _| default_button_style());

        row![
            container(cb).width(28),
            container(name_input).width(130),
            container(value_input).width(Length::Fill),
            row![save_btn, cancel_btn]
                .spacing(4)
                .align_y(iced::Alignment::Center),
        ]
        .spacing(8)
        .padding([6, 8])
        .align_y(iced::Alignment::Center)
        .into()
    }

    /// Row actions: shows context menu (open/copy) if active, otherwise edit/delete.
    fn view_row_actions_or_context(
        &self,
        kind: ItemKind,
        name: &str,
        current_value: &str,
        current_aliases: &str,
    ) -> Element<'_, Message> {
        if self.has_context_menu(kind, name) {
            let ctx = self.context_menu.as_ref().unwrap();
            let urls = &ctx.urls;

            if urls.is_empty() {
                return text("no urls").size(12).color(colors::TEXT_DIM).into();
            }

            let open_msg = if urls.len() == 1 {
                Message::OpenUrl(urls[0].clone())
            } else {
                Message::OpenUrls(urls.clone())
            };
            let copy_text = urls.join("\n");

            let open_btn = button(text("open").size(12).color(colors::CYAN))
                .on_press(open_msg)
                .padding([2, 8])
                .style(|_, _| context_button_style());

            let copy_btn = button(text("copy").size(12).color(colors::CYAN))
                .on_press(Message::CopyUrl(copy_text))
                .padding([2, 8])
                .style(|_, _| context_button_style());

            return row![open_btn, copy_btn]
                .spacing(4)
                .align_y(iced::Alignment::Center)
                .into();
        }

        let edit_btn = button(text("edit").size(12).color(colors::TEXT))
            .on_press(Message::StartRowEdit(
                kind,
                name.to_string(),
                name.to_string(),
                current_value.to_string(),
                current_aliases.to_string(),
            ))
            .padding([2, 8])
            .style(|_, _| default_button_style());

        let delete_btn = button(text("delete").size(12).color(colors::RED))
            .on_press(Message::RequestDelete(kind, name.to_string()))
            .padding([2, 8])
            .style(|_, _| danger_button_style());

        row![edit_btn, delete_btn]
            .spacing(4)
            .align_y(iced::Alignment::Center)
            .into()
    }

    fn view_confirm_modal<'a>(&self, confirm: &'a ConfirmState) -> Element<'a, Message> {
        let title = text(&confirm.title).size(16).color(colors::TEXT_BRIGHT);
        let message = text(&confirm.message).size(13).color(colors::TEXT);

        let cancel_btn = button(text("cancel").size(13).color(colors::TEXT))
            .on_press(Message::ConfirmNo)
            .padding([6, 16])
            .style(|_, _| default_button_style());

        let confirm_btn = button(text("delete").size(13).color(colors::RED))
            .on_press(Message::ConfirmYes)
            .padding([6, 16])
            .style(|_, _| button::Style {
                background: Some(iced::Background::Color(colors::RED_BG)),
                border: iced::Border {
                    color: colors::RED,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                text_color: colors::RED,
                ..Default::default()
            });

        let modal_content = container(
            column![
                title,
                message,
                row![
                    iced::widget::Space::new().width(Length::Fill),
                    cancel_btn,
                    confirm_btn
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            ]
            .spacing(12),
        )
        .padding(24)
        .max_width(400)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb(
                0.08, 0.08, 0.13,
            ))),
            border: iced::Border {
                color: colors::BORDER,
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        });

        center(modal_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgba(
                    0.0, 0.0, 0.0, 0.7,
                ))),
                ..Default::default()
            })
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn title(&self) -> String {
        "bookmarks".into()
    }
}

// -- Style helpers -----------------------------------------------------------

fn input_style(status: text_input::Status) -> text_input::Style {
    let border_color = match status {
        text_input::Status::Focused { .. } => colors::BORDER_FOCUS,
        _ => colors::BORDER,
    };
    text_input::Style {
        background: iced::Background::Color(colors::BG_INPUT),
        border: iced::Border {
            color: border_color,
            width: 1.0,
            radius: 4.0.into(),
        },
        icon: colors::TEXT_DIM,
        placeholder: colors::TEXT_DIM,
        value: colors::TEXT_BRIGHT,
        selection: colors::PURPLE,
    }
}

fn edit_input_style(_status: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: iced::Background::Color(colors::BG_INPUT),
        border: iced::Border {
            color: colors::PURPLE,
            width: 1.0,
            radius: 3.0.into(),
        },
        icon: colors::TEXT_DIM,
        placeholder: colors::TEXT_DIM,
        value: colors::TEXT_BRIGHT,
        selection: colors::PURPLE,
    }
}

fn tab_button_style(is_active: bool, _status: button::Status) -> button::Style {
    button::Style {
        background: if is_active {
            Some(iced::Background::Color(colors::TAB_ACTIVE_BG))
        } else {
            None
        },
        border: iced::Border {
            color: if is_active {
                colors::PURPLE
            } else {
                colors::BORDER
            },
            width: 1.0,
            radius: 4.0.into(),
        },
        text_color: if is_active {
            colors::PURPLE
        } else {
            colors::TEXT
        },
        ..Default::default()
    }
}

fn default_button_style() -> button::Style {
    button::Style {
        background: None,
        border: iced::Border {
            color: colors::BORDER,
            width: 1.0,
            radius: 4.0.into(),
        },
        text_color: colors::TEXT,
        ..Default::default()
    }
}

fn danger_button_style() -> button::Style {
    button::Style {
        background: None,
        border: iced::Border {
            color: colors::RED_BORDER,
            width: 1.0,
            radius: 4.0.into(),
        },
        text_color: colors::RED,
        ..Default::default()
    }
}

fn add_button_style() -> button::Style {
    button::Style {
        background: Some(iced::Background::Color(colors::BG_INPUT)),
        border: iced::Border {
            color: colors::BORDER,
            width: 1.0,
            radius: 4.0.into(),
        },
        text_color: colors::PURPLE,
        ..Default::default()
    }
}

fn checkbox_style() -> checkbox::Style {
    checkbox::Style {
        background: iced::Background::Color(colors::BG_INPUT),
        icon_color: colors::PURPLE,
        border: iced::Border {
            color: colors::BORDER,
            width: 1.0,
            radius: 3.0.into(),
        },
        text_color: Some(colors::TEXT),
    }
}

fn context_button_style() -> button::Style {
    button::Style {
        background: Some(iced::Background::Color(colors::BG_INPUT)),
        border: iced::Border {
            color: colors::CYAN,
            width: 1.0,
            radius: 4.0.into(),
        },
        text_color: colors::CYAN,
        ..Default::default()
    }
}

fn link_cell_style(status: button::Status) -> button::Style {
    button::Style {
        background: match status {
            button::Status::Hovered => Some(iced::Background::Color(colors::BG_HOVER)),
            _ => None,
        },
        border: iced::Border {
            radius: 3.0.into(),
            ..Default::default()
        },
        text_color: colors::PURPLE,
        ..Default::default()
    }
}

fn rule_style() -> iced::widget::rule::Style {
    iced::widget::rule::Style {
        color: iced::Color::from_rgb(0.14, 0.14, 0.22),
        radius: 0.0.into(),
        fill_mode: iced::widget::rule::FillMode::Full,
        snap: false,
    }
}

// -- Entry point -------------------------------------------------------------

fn load_icon() -> Option<iced::window::Icon> {
    let png_bytes = include_bytes!("../assets/icon.png");
    let decoder = png::Decoder::new(png_bytes.as_slice());
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());

    // Convert RGB to RGBA if needed
    let rgba = if info.color_type == png::ColorType::Rgb {
        let mut rgba = Vec::with_capacity(buf.len() / 3 * 4);
        for chunk in buf.chunks(3) {
            rgba.extend_from_slice(chunk);
            rgba.push(255);
        }
        rgba
    } else {
        buf
    };

    iced::window::icon::from_rgba(rgba, info.width, info.height).ok()
}

pub fn run_app(storage: Box<dyn Storage>) -> iced::Result {
    use std::cell::RefCell;
    let storage = RefCell::new(Some(storage));

    let window_settings = iced::window::Settings {
        icon: load_icon(),
        ..Default::default()
    };

    iced::application(
        move || {
            let s = storage
                .borrow_mut()
                .take()
                .expect("boot called more than once");
            Bookmarks::new(s)
        },
        Bookmarks::update,
        Bookmarks::view,
    )
    .title(Bookmarks::title)
    .theme(Bookmarks::theme)
    .antialiasing(true)
    .window(window_settings)
    .window_size(Size::new(720.0, 800.0))
    .run()
}
