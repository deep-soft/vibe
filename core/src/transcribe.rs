use crate::audio;
use crate::config::{DiarizeOptions, TranscribeOptions};
use crate::transcript::{Segment, Transcript};
use eyre::{bail, eyre, Context, OptionExt, Result};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
pub use whisper_rs::SegmentCallbackData;
pub use whisper_rs::WhisperContext;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContextParameters};

type ProgressCallbackType = once_cell::sync::Lazy<Mutex<Option<Box<dyn Fn(i32) + Send + Sync>>>>;
static PROGRESS_CALLBACK: ProgressCallbackType = once_cell::sync::Lazy::new(|| Mutex::new(None));

pub fn create_context(model_path: &Path, gpu_device: Option<i32>) -> Result<WhisperContext> {
    whisper_rs::install_whisper_tracing_trampoline();
    tracing::debug!("open model...");
    if !model_path.exists() {
        bail!("whisper file doesn't exist")
    }
    let mut ctx_params = WhisperContextParameters::default();
    if !env!("CUDA_VERSION").is_empty() || !env!("ROCM_VERSION").is_empty() {
        // Nvidia or AMD
        ctx_params.use_gpu = true;
    }
    // set GPU device number from preference
    if let Some(gpu_device) = gpu_device {
        ctx_params.gpu_device = gpu_device;
    }
    tracing::debug!("gpu device: {:?}", ctx_params.gpu_device);
    tracing::debug!("use gpu: {:?}", ctx_params.use_gpu);
    let model_path = model_path.to_str().ok_or_eyre("can't convert model option to str")?;
    tracing::debug!("creating whisper context with model path {}", model_path);
    let ctx_unwind_result = catch_unwind(AssertUnwindSafe(|| {
        WhisperContext::new_with_params(model_path, ctx_params).context("failed to open model")
    }));
    match ctx_unwind_result {
        Err(error) => {
            bail!("create whisper context crash: {:?}", error)
        }
        Ok(ctx_result) => {
            let ctx = ctx_result?;
            tracing::debug!("created context successfuly");
            Ok(ctx)
        }
    }
}

pub fn create_normalized_audio(source: PathBuf) -> Result<PathBuf> {
    let out_path = tempfile::Builder::new()
        .suffix(".wav")
        .tempfile()?
        .into_temp_path()
        .to_path_buf();
    audio::normalize(source, out_path.clone())?;
    Ok(out_path)
}

pub fn transcribe(
    ctx: &WhisperContext,
    options: &TranscribeOptions,
    progress_callback: Option<Box<dyn Fn(i32) + Send + Sync>>,
    new_segment_callback: Option<Box<dyn Fn(Segment)>>,
    abort_callback: Option<Box<dyn Fn() -> bool>>,
    #[allow(unused_variables)] diarize_options: Option<DiarizeOptions>,
) -> Result<Transcript> {
    tracing::debug!("Transcribe called with {:?}", options);

    if !PathBuf::from(options.path.clone()).exists() {
        bail!("audio file doesn't exist")
    }

    if let Some(callback) = progress_callback {
        let mut guard = PROGRESS_CALLBACK.lock().map_err(|e| eyre!("{:?}", e))?;
        *guard = Some(Box::new(callback));
    }

    let out_path = create_normalized_audio(options.path.clone().into())?;
    let original_samples = audio::parse_wav_file(&out_path)?;
    let diarize_segments = pyannote_rs::segment::segment(&original_samples, 16000, Path::new("segmentation-3.0.onnx"))
        .map_err(|e| eyre!("{:?}", e))?;
    println!("segments: {:?}", diarize_segments);

    let mut samples = vec![0.0f32; original_samples.len()];
    whisper_rs::convert_integer_to_float_audio(&original_samples, &mut samples)?;

    let mut state = ctx.create_state().context("failed to create key")?;

    let mut params = FullParams::new(SamplingStrategy::default());
    tracing::debug!("set language to {:?}", options.lang);

    let mut embedding_extractor =
        pyannote_rs::embedding::EmbeddingExtractor::new(Path::new("wespeaker_en_voxceleb_CAM++.onnx")).unwrap();
    let mut embedding_manager = pyannote_rs::identify::EmbeddingManager::new(6);

    if let Some(true) = options.word_timestamps {
        params.set_token_timestamps(true);
        params.set_split_on_word(true);
        params.set_max_len(options.max_sentence_len.unwrap_or(1));
    }

    if let Some(true) = options.translate {
        params.set_translate(true);
    }
    if options.lang.is_some() {
        params.set_language(options.lang.as_deref());
    }

    params.set_print_special(false);
    params.set_print_progress(true);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_suppress_blank(true);
    params.set_token_timestamps(true);

    if let Some(temperature) = options.temperature {
        tracing::debug!("setting temperature to {temperature}");
        params.set_temperature(temperature);
    }

    if let Some(max_text_ctx) = options.max_text_ctx {
        tracing::debug!("setting n_max_text_ctx to {}", max_text_ctx);
        params.set_n_max_text_ctx(max_text_ctx)
    }

    // handle args
    if let Some(init_prompt) = options.init_prompt.to_owned() {
        tracing::debug!("setting init prompt to {init_prompt}");
        params.set_initial_prompt(&init_prompt);
    }

    if let Some(n_threads) = options.n_threads {
        tracing::debug!("setting n threads to {n_threads}");
        params.set_n_threads(n_threads);
    }

    // params.set_single_segment(true);

    let mut segments = Vec::new();

    for diarize_segment in diarize_segments {
        let start_f64 = diarize_segment.0 * (16000 as f64);
        let end_f64 = diarize_segment.1 * (16000 as f64);

        // Ensure indices are within bounds
        let start_idx = start_f64.min((samples.len() - 1) as f64) as usize;
        let end_idx = end_f64.min(samples.len() as f64) as usize;
        let segment_samples = &samples[start_idx..end_idx];

        tracing::debug!("set start time...");
        let st = std::time::Instant::now();
        tracing::debug!("setting state full...");
        state.full(params.clone(), &segment_samples).context("failed to transcribe")?;
        let _et = std::time::Instant::now();

        tracing::debug!("getting segments count...");
        let num_segments = state.full_n_segments().context("failed to get number of segments")?;
        if num_segments == 0 {
            tracing::warn!(
                "no segments found in {} {}. skipping...",
                diarize_segment.0,
                diarize_segment.1
            );
            continue;
        }
        tracing::debug!("found {} sentence segments", num_segments);

        tracing::debug!("looping segments...");
        for s in 0..num_segments {
            let text = state.full_get_segment_text_lossy(s).context("failed to get segment")?;
            let mut speaker = "?".to_string();
            match embedding_extractor.compute(&segment_samples) {
                Ok(embedding_result) => {
                    speaker = embedding_manager
                        .get_speaker(embedding_result, 0.5)
                        .map(|r| r.to_string())
                        .unwrap_or("?".into());
                }
                Err(error) => {
                    println!("error: {:?}", error);
                }
            }
            let segment = Segment {
                text,
                start: diarize_segment.0 as i64,
                stop: diarize_segment.1 as i64,
                speaker: Some(speaker),
            };
            if let Some(ref callback) = new_segment_callback {
                callback(segment.clone());
            }
            segments.push(segment);
        }
    }

    #[allow(unused_mut)]
    let mut transcript = Transcript {
        segments,
        processing_time_sec: 0,
    };

    // cleanup
    std::fs::remove_file(out_path)?;

    Ok(transcript)
}
