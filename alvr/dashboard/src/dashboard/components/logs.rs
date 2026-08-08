use crate::language::tr;
use alvr_common::LogSeverity;
use alvr_events::{Event, EventType};
use alvr_gui_common::theme::log_colors;
use alvr_session::{RawEventsConfig, Settings};
use eframe::{
    egui::{ComboBox, Grid, OpenUrl, OutputCommand, RichText, ScrollArea, TextEdit, Ui},
    epaint::Color32,
};
use settings_schema::Switch;
use std::collections::VecDeque;

struct Entry {
    color: Color32,
    severity: Option<LogSeverity>,
    timestamp: String,
    ty: String,
    message: String,
    search_index: String,
}

pub struct LogsTab {
    raw_events_config: Switch<RawEventsConfig>,
    entries: VecDeque<Entry>,
    log_limit: usize,
    search_query: String,
    severity_filter: Option<LogSeverity>,
}

impl LogsTab {
    pub fn new() -> Self {
        Self {
            raw_events_config: Switch::Enabled(RawEventsConfig {
                hide_spammy_events: false,
            }),
            entries: VecDeque::new(),
            log_limit: 1000,
            search_query: String::new(),
            severity_filter: None,
        }
    }

    pub fn update_settings(&mut self, settings: &Settings) {
        self.raw_events_config = settings.extra.logging.show_raw_events.clone();
    }

    pub fn push_event(&mut self, event: Event) {
        let severity = if let EventType::Log(entry) = &event.event_type {
            Some(entry.severity)
        } else {
            None
        };
        let color = if let EventType::Log(entry) = &event.event_type {
            Some(match entry.severity {
                LogSeverity::Error => log_colors::ERROR_LIGHT,
                LogSeverity::Warning => log_colors::WARNING_LIGHT,
                LogSeverity::Info => log_colors::INFO_LIGHT,
                LogSeverity::Debug => log_colors::DEBUG_LIGHT,
            })
        } else if let Switch::Enabled(config) = &self.raw_events_config {
            (!config.hide_spammy_events
                || !matches!(
                    event.event_type,
                    EventType::StatisticsSummary(_)
                        | EventType::GraphStatistics(_)
                        | EventType::Tracking(_)
                ))
            .then_some(log_colors::EVENT_LIGHT)
        } else {
            None
        };

        if let Some(color) = color {
            let timestamp = event.timestamp.clone();
            let ty = event.event_type_string();
            let message = event.message();
            let search_index = format!("{timestamp} {ty} {message}").to_lowercase();

            self.entries.push_back(Entry {
                color,
                severity,
                timestamp,
                ty,
                message,
                search_index,
            });

            if self.entries.len() > self.log_limit {
                self.entries.pop_front();
            }
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui.button(tr("Copy all")).clicked() {
                ui.output_mut(|out| {
                    out.commands
                        .push(OutputCommand::CopyText(self.entries.iter().fold(
                            String::new(),
                            |acc, entry| {
                                format!(
                                    "{}{} [{}] {}\n",
                                    acc, entry.timestamp, entry.ty, entry.message
                                )
                            },
                        )));
                })
            }
            if ui.button(tr("Open logs directory")).clicked() {
                let log_dir = crate::get_filesystem_layout().log_dir;
                ui.ctx().open_url(OpenUrl::same_tab(format!(
                    "file://{}",
                    log_dir.to_string_lossy()
                )));
            }
            if ui.button(tr("Clear all")).clicked() {
                self.entries.clear();
            }

            ui.add_sized(
                [240.0, 24.0],
                TextEdit::singleline(&mut self.search_query).hint_text(tr("Search logs")),
            );
            if !self.search_query.is_empty() && ui.button(tr("Clear search")).clicked() {
                self.search_query.clear();
            }

            ComboBox::from_id_salt("logs-severity-filter")
                .selected_text(match self.severity_filter {
                    None => tr("All levels"),
                    Some(LogSeverity::Error) => tr("Error"),
                    Some(LogSeverity::Warning) => tr("Warning"),
                    Some(LogSeverity::Info) => tr("Info"),
                    Some(LogSeverity::Debug) => tr("Debug"),
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.severity_filter, None, tr("All levels"));
                    ui.selectable_value(
                        &mut self.severity_filter,
                        Some(LogSeverity::Error),
                        tr("Error"),
                    );
                    ui.selectable_value(
                        &mut self.severity_filter,
                        Some(LogSeverity::Warning),
                        tr("Warning"),
                    );
                    ui.selectable_value(
                        &mut self.severity_filter,
                        Some(LogSeverity::Info),
                        tr("Info"),
                    );
                    ui.selectable_value(
                        &mut self.severity_filter,
                        Some(LogSeverity::Debug),
                        tr("Debug"),
                    );
                });

            let query = self.search_query.trim().to_lowercase();
            let matches = |entry: &&Entry| {
                (query.is_empty() || entry.search_index.contains(&query))
                    && self
                        .severity_filter
                        .is_none_or(|severity| entry.severity == Some(severity))
            };
            let match_count = self.entries.iter().filter(matches).count();
            ui.label(format!(
                "{}: {match_count}/{}",
                tr("Matches"),
                self.entries.len()
            ));
        });

        let query = self.search_query.trim().to_lowercase();
        ScrollArea::both()
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                Grid::new(0)
                    .spacing((10.0, 2.0))
                    .num_columns(3)
                    .striped(true)
                    .show(ui, |ui| {
                        for entry in self.entries.iter().filter(|entry| {
                            (query.is_empty() || entry.search_index.contains(&query))
                                && self
                                    .severity_filter
                                    .is_none_or(|severity| entry.severity == Some(severity))
                        }) {
                            ui.colored_label(
                                entry.color,
                                RichText::new(&entry.timestamp).size(12.0),
                            );
                            ui.colored_label(entry.color, RichText::new(&entry.ty).size(12.0));
                            ui.colored_label(entry.color, RichText::new(&entry.message).size(12.0));

                            ui.end_row();
                        }
                    });
            });
    }
}
