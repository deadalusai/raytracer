use std::sync::Arc;

use eframe::egui::{self, Spinner, TextureHandle};
use raytracer_samples::scene::{ SceneFactory, SceneControlCollection };

use crate::frame_history::FrameHistory;
use crate::job_complete::RenderJobCompleteState;
use crate::job_constructing::{RenderJobConstructingState, start_render_job_construction};
use crate::job_running::RenderJobRunningState;
use crate::logger_view::{logger_view};
use crate::thread_stats::ThreadStats;
use crate::settings::{ SettingsWidget, Settings };

pub struct App {
    // Persistent state
    settings: Settings,
    // Configuration
    scene_factories: Vec<Arc<dyn SceneFactory + Send + Sync>>,
    scene_configs: Vec<SceneControlCollection>,
    // Panel state
    settings_open: bool,
    logs_open: bool,
    // Temporal state
    frame_history: FrameHistory,
    state: AppState,
}

pub enum AppState {
    None,
    RenderJobConstructing(RenderJobConstructingState),
    RenderJobRunning(RenderJobRunningState),
    RenderJobComplete(RenderJobCompleteState),
    Error(String),
}

pub enum AppStateUpdateResult {
    None,
    RequestRefresh,
    TransitionToNewState(AppState),
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {

        // Persistent state
        let settings = match cc.storage {
            None => Settings::default(),
            Some(s) => eframe::get_value(s, eframe::APP_KEY).unwrap_or_default(),
        };

        let scene_factories = raytracer_samples::make_sample_scene_factories();
        let scene_configs = scene_factories.iter().map(|f| f.create_controls()).collect();

        App {
            settings,
            // Configuration
            scene_factories,
            scene_configs,
            // Panel state:
            settings_open: true,
            logs_open: true,
            // Temporal state
            frame_history: FrameHistory::default(),
            state: AppState::None,
        }
    }

    fn start_new_job(&mut self) {

        // Stop running worker threads if an existing job is in progress
        if let AppState::RenderJobRunning(running) = &self.state {
            running.stop();
        }

        let scene_factory = self.scene_factories[self.settings.scene].clone();
        let scene_config = self.scene_configs[self.settings.scene].collect_configuration();

        let state = start_render_job_construction(self.settings.clone(), scene_config, scene_factory);
        self.state = AppState::RenderJobConstructing(state);
    }

    /// Runs internal state update logic, and may transition
    /// the app into a new state.
    fn update_state(&mut self, ctx: &eframe::egui::Context) {
        let result = match &mut self.state {
            AppState::RenderJobConstructing(state) => state.update(),
            AppState::RenderJobRunning(state) => state.update(ctx),
            _ => AppStateUpdateResult::None
        };

        match result {
            AppStateUpdateResult::None => {},
            AppStateUpdateResult::RequestRefresh => {
                ctx.request_repaint();
            },
            AppStateUpdateResult::TransitionToNewState(state) => {
                self.state = state;
                ctx.request_repaint();
            },
        }
    }

    fn output_image(&self, tex: &TextureHandle) -> egui::Image<'_> {
        if self.settings.scale_render_to_window {
            egui::Image::new(tex).fit_to_fraction(egui::vec2(1.0, 1.0))
        }
        else {
            egui::Image::new(tex).fit_to_original_size(1.0)
        }
    }

    fn resolve_thread_stats(&self) -> Option<Box<dyn Iterator<Item = ThreadStats> + '_>> {
        match &self.state {
            AppState::RenderJobRunning(state) => Some(Box::new(state.job.thread_stats())),
            AppState::RenderJobComplete(state) => Some(Box::new(state.thread_stats.iter().cloned())),
            _ => None
        }
    }
}

impl eframe::App for App {

    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.settings);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {

        self.update_state(ctx);

        ctx.set_visuals(egui::Visuals::dark());

        self.frame_history.on_new_frame(ctx.input(|s| s.time), frame.info().cpu_usage);

        egui::TopBottomPanel::top("top bar")
            .frame(egui::Frame::new().inner_margin(4))
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.visuals_mut().button_frame = false;
                    ui.toggle_value(&mut self.settings_open, "⚙ Settings");
                    ui.toggle_value(&mut self.logs_open, "🗎 Logs");
                });
            });

        egui::TopBottomPanel::bottom("logs")
            .resizable(true)
            .frame(egui::Frame::new().inner_margin(4).outer_margin(4))
            .show_animated(ctx, self.logs_open, |ui| {
                logger_view(ui);
            });

        egui::SidePanel::left("settings")
            .resizable(true)
            .min_width(300.0)
            .show_animated(ctx, self.settings_open, |ui| {
                ui.add_space(4.0);
                ui.vertical_centered(|ui| {
                    ui.heading("Settings");
                });

                ui.separator();
                egui::ScrollArea::vertical()
                    .animated(false)
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        ui.add(SettingsWidget::new(&mut self.settings, &mut self.scene_configs));
                        ui.separator();
                        ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {
                            if ui.button("Start render").clicked() {
                                self.start_new_job();
                            }
                        });
                        if let Some(thread_stats) = self.resolve_thread_stats() {
                            ui.separator();
                            for stats in thread_stats {
                                ui.add(stats);
                            }
                        }
                        ui.separator();
                        self.frame_history.ui(ui);
                    });
            });

        egui::CentralPanel::default()
            .show(ctx, |ui| {
                ui.centered_and_justified(|ui| {
                    match &self.state {
                        AppState::None => {
                            ui.label("Press 'Start render' to start")
                        },
                        AppState::RenderJobConstructing(_) => {
                            ui.add(Spinner::new())
                        },
                        AppState::RenderJobRunning(state) => {
                            match state.output_tex.as_ref() {
                                Some(tex) => ui.add(self.output_image(tex)),
                                None => ui.spinner(),
                            }
                        },
                        AppState::RenderJobComplete(state) => {
                            ui.add(self.output_image(&state.output_tex))
                        },
                        AppState::Error(error) => {
                            ui.label(error)
                        },
                    }
                });
            });
    }
}
