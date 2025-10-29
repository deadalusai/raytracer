use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Instant, Duration};

use log::info;

use cancellation::{CancellationToken, CancellationTokenSource};
use flume::{Receiver, Sender};
use raytracer_impl::implementation::{RenderSettings, Scene};
use raytracer_impl::viewport::{RenderChunk};

use crate::rgba::{RgbaBuffer, v3_to_rgba};
use crate::thread_stats::ThreadStats;

const RNG_SEED: u64 = 12345;

pub struct RenderJob {
    pub render_args: Arc<(Scene, RenderSettings)>,
    pub chunks: Vec<RenderChunk>,
    pub next_chunk_index: usize,
    pub start_time: Instant,
    pub render_time_secs: f64,
    pub completed_chunk_count: usize,
    pub updates: Vec<([usize; 2], RgbaBuffer)>,
    pub worker_handle: RenderJobWorkerHandle,
}

#[derive(Eq, PartialEq)]
pub enum RenderJobUpdateResult {
    Updated,
    ErrorRenderThreadsStopped
}

impl RenderJob {
    pub fn is_work_completed(&self) -> bool {
        self.completed_chunk_count >= self.chunks.len()
    }

    pub fn update(&mut self) -> RenderJobUpdateResult {
        use RenderThreadMessage::*;

        // Poll for completed work
        while let Ok(result) = self.worker_handle.result_receiver.try_recv() {
            match result {
                Ready => {}, // Worker thread ready to go.
                FrameUpdated(chunk, buf) => {
                    // Chunk ready to blit to texture
                    self.updates.push(([chunk.left, chunk.top], buf));
                },
                FrameCompleted(id, elapsed) => {
                    // Update stats
                    let thread = &mut self.worker_handle.thread_handles[id as usize];
                    thread.total_time_secs += duration_total_secs(elapsed);
                    thread.total_chunks_rendered += 1;
                    self.completed_chunk_count += 1;
                },
                Terminated => {}, // Worker halted
            }
        }

        // Refill the the work queue
        use flume::TrySendError;
        while self.next_chunk_index < self.chunks.len() {
            let chunk = &self.chunks[self.next_chunk_index];
            let work = RenderWork(chunk.clone(), self.render_args.clone());
            if let Err(err) = self.worker_handle.work_sender.try_send(work) {
                match err {
                    TrySendError::Full(_) => {
                        // Queue full, try again later
                        break;
                    }
                    TrySendError::Disconnected(_) => {
                        return RenderJobUpdateResult::ErrorRenderThreadsStopped
                    }
                }
            }
            self.next_chunk_index += 1;
        }

        if !self.is_work_completed() {
            // Update timer
            self.render_time_secs = duration_total_secs(self.start_time.elapsed());
        }

        RenderJobUpdateResult::Updated
    }

    #[allow(unused)]
    fn reset(&mut self) {
        self.next_chunk_index = 0;
        self.completed_chunk_count = 0;
    }

    pub fn thread_stats(&self) -> impl Iterator<Item=ThreadStats> {
        self.worker_handle.thread_handles.iter()
            .map(|thread| ThreadStats {
                id: thread.id,
                total_chunks_rendered: thread.total_chunks_rendered,
                total_time_secs: thread.total_time_secs,
            })
    }
}

fn duration_total_secs(elapsed: Duration) -> f64 {
    elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 * 1e-9
}

// A message from the master thread to a worker
#[derive(Clone)]
pub struct RenderWork(pub RenderChunk, pub Arc<(Scene, RenderSettings)>);

// A message from a worker thread to the master thread
#[derive(Clone)]
pub enum RenderThreadMessage {
    Ready,
    FrameUpdated(RenderChunk, RgbaBuffer),
    FrameCompleted(ThreadId, Duration),
    Terminated
}

type BoxError = Box<dyn std::error::Error + 'static>;

fn start_render_thread(
    id: ThreadId,
    cancellation_token: &CancellationToken,
    work_receiver: &Receiver<RenderWork>,
    result_sender: &Sender<RenderThreadMessage>
) -> Result<(), BoxError> {
    use RenderThreadMessage::*;
    use rand::SeedableRng;
    use rand_xorshift::XorShiftRng;

    result_sender.send(Ready)?;

    // Receive messages
    for RenderWork(chunk, args) in work_receiver.into_iter() {
        if cancellation_token.is_canceled() {
            return Ok(());
        }
        // Paint in-progress chunks green
        let mut buffer = RgbaBuffer::new(chunk.width, chunk.height);
        let green = v3_to_rgba(raytracer_impl::types::V3(0.0, 0.58, 0.0));
        for p in chunk.iter_pixels() {
            buffer.put_pixel(p.chunk_pos, green);
        }
        result_sender.send(FrameUpdated(chunk.clone(), buffer.clone()))?;
        // Using the same seeded RNG for every frame makes every run repeatable
        let mut rng = XorShiftRng::seed_from_u64(RNG_SEED);
        // Render the scene chunk
        let (scene, render_settings) = args.as_ref();
        let time = Instant::now();
        // For each x, y coordinate in this view chunk, cast a ray.
        for p in chunk.iter_pixels() {
            if cancellation_token.is_canceled() {
                return Ok(());
            }
            // Convert to view-relative coordinates
            let color = raytracer_impl::implementation::cast_rays_into_scene(scene, render_settings, p.view_pos, &mut rng);
            buffer.put_pixel(p.chunk_pos, v3_to_rgba(color));
        }
        let elapsed = time.elapsed();
        // Send final frame and results
        result_sender.send(FrameUpdated(chunk, buffer))?;
        result_sender.send(FrameCompleted(id, elapsed))?;
    }

    Ok(())
}

type ThreadId = u32;

#[allow(unused)]
pub struct RenderThread {
    pub id: ThreadId,
    pub handle: JoinHandle<()>,
    pub total_time_secs: f64,
    pub total_chunks_rendered: u32,
}

pub struct RenderJobWorkerHandle {
    pub cts: CancellationTokenSource,
    pub work_sender: Sender<RenderWork>,
    pub result_receiver: Receiver<RenderThreadMessage>,
    pub thread_handles: Vec<RenderThread>,
}

pub fn start_background_render_threads(render_thread_count: u32) -> RenderJobWorkerHandle {
    let cts = CancellationTokenSource::new();
    let (work_sender, work_receiver) = flume::bounded(render_thread_count as usize);
    let (result_sender, result_receiver) = flume::unbounded();

    let thread_handles = (0..render_thread_count)
        .map(|id| {
            let cancellation_token = cts.token().clone();
            let work_receiver = work_receiver.clone();
            let result_sender = result_sender.clone();
            let work = move || {
                if let Err(err) = start_render_thread(id, &cancellation_token, &work_receiver, &result_sender) {
                    info!("Thread {id} terminated due to error: {err}");
                }
                // Notify master thread that we've terminated.
                // NOTE: There may be nobody listening...
                result_sender.send(RenderThreadMessage::Terminated).ok();
            };
            let handle = std::thread::Builder::new()
                .name(format!("Render Thread {id}"))
                .spawn(work)
                .expect("failed to spawn render thread");

            RenderThread {
                id,
                handle,
                total_time_secs: 0.0,
                total_chunks_rendered: 0,
            }
        })
        .collect::<Vec<_>>();

    RenderJobWorkerHandle {
        cts,
        work_sender,
        result_receiver,
        thread_handles,
    }
}
