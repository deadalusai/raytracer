use std::sync::Arc;

use eframe::egui::{self, Spinner};
use raytracer_samples::scene::{ SceneFactory, SceneControlCollection };

use crate::frame_history::FrameHistory;
use crate::job_constructing::{RenderJobConstructingState, start_render_job_construction};
use crate::job_running::RenderJobRunningState;
use crate::thread_stats::ThreadStats;
use crate::settings::{ SettingsWidget, Settings };

pub struct App {
    // Persistent state
    settings: Settings,
    // Configuration
    scene_factories: Vec<Arc<dyn SceneFactory + Send + Sync>>,
    scene_configs: Vec<SceneControlCollection>,
    // Temporal state
    frame_history: FrameHistory,
    state: AppState,
}

pub enum AppState {
    None,
    RenderJobConstructing(RenderJobConstructingState),
    RenderJobRunning(RenderJobRunningState),
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

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                match &self.state {
                    AppState::None => {
                        ui.label("Press 'Start render' to start");
                    },
                    AppState::RenderJobConstructing(_) => {
                        ui.add(Spinner::new());
                    },
                    AppState::RenderJobRunning(state) => {
                        match state.output_tex.as_ref() {
                            Some(output_texture) => {
                                ui.add(
                                    if self.settings.scale_render_to_window {
                                        egui::Image::new(output_texture).fit_to_fraction(egui::vec2(1.0, 1.0))
                                    }
                                    else {
                                        egui::Image::new(output_texture).fit_to_original_size(1.0)
                                    }
                                );
                            }
                            None => {
                                ui.spinner();
                            }
                        }
                    },
                    AppState::Error(error) => {
                        ui.label(error);
                    },
                }
            });
        });

        // Settings UI
        egui::Window::new("Settings")
            .resizable(false)
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.add(SettingsWidget::new(&mut self.settings, &mut self.scene_configs));
                ui.separator();

                ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {
                    if ui.button("Start render").clicked() {
                        self.start_new_job();
                    }
                });

                if let AppState::RenderJobRunning(state) = &self.state {
                    for thread in state.job.worker_handle.thread_handles.iter() {
                        ui.add(ThreadStats {
                            id: thread.id,
                            total_chunks_rendered: thread.total_chunks_rendered,
                            total_time_secs: thread.total_time_secs,
                        });
                    }
                }

                self.frame_history.ui(ui);
            });
    }
}
