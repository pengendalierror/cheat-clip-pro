use crate::model::{cache_dir, Aspect, Clip, Cue, Source};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

pub fn tool_exists(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

pub fn probe_duration(path: &Path) -> Result<f64, String> {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=nw=1:nk=1",
        ])
        .arg(path)
        .output()
        .map_err(|err| format!("ffprobe: {err}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse::<f64>()
        .map_err(|_| "ffprobe tidak mengembalikan durasi".to_string())
}

pub fn resolve_source(input: &str) -> Result<Source, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Isi path file atau tautan dulu.".into());
    }
    let path = PathBuf::from(trimmed);
    if path.is_file() {
        let duration = probe_duration(&path)?;
        let title = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Video".into());
        return Ok(Source {
            path: path.to_string_lossy().to_string(),
            title,
            duration,
            origin: path.to_string_lossy().to_string(),
        });
    }
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return download_url(trimmed);
    }
    Err(format!("Bukan file dan bukan tautan: {trimmed}"))
}

fn download_url(url: &str) -> Result<Source, String> {
    if !tool_exists("yt-dlp") {
        return Err("yt-dlp belum terpasang. Buka file lokal, atau pasang yt-dlp.".into());
    }
    let dir = cache_dir();
    fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    let title = Command::new("yt-dlp")
        .args(["--no-playlist", "--print", "title", "--skip-download", url])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Video".into());
    let template = dir.join("source.%(ext)s");
    let status = Command::new("yt-dlp")
        .args([
            "--no-playlist",
            "-f",
            "bv*[ext=mp4]+ba[ext=m4a]/b[ext=mp4]/best",
            "--merge-output-format",
            "mp4",
            "-o",
        ])
        .arg(&template)
        .arg(url)
        .status()
        .map_err(|err| format!("yt-dlp: {err}"))?;
    if !status.success() {
        return Err("yt-dlp tidak bisa mengunduh tautan itu.".into());
    }
    let file = ["source.mp4", "source.mkv", "source.webm"]
        .into_iter()
        .map(|name| dir.join(name))
        .find(|path| path.is_file())
        .ok_or_else(|| "yt-dlp selesai, tetapi file videonya tidak ketemu.".to_string())?;
    let duration = probe_duration(&file)?;
    Ok(Source {
        path: file.to_string_lossy().to_string(),
        title,
        duration,
        origin: url.to_string(),
    })
}

pub fn load_cues(subtitle_path: &str, video_path: &str) -> Vec<Cue> {
    let explicit = PathBuf::from(subtitle_path.trim());
    if explicit.is_file() {
        if let Ok(text) = fs::read_to_string(&explicit) {
            let cues = parse_srt(&text);
            if !cues.is_empty() {
                return cues;
            }
        }
    }
    let video = PathBuf::from(video_path);
    if let Some(stem) = video.file_stem() {
        let sibling = video.with_file_name(format!("{}.srt", stem.to_string_lossy()));
        if let Ok(text) = fs::read_to_string(sibling) {
            return parse_srt(&text);
        }
    }
    Vec::new()
}

pub fn parse_srt(text: &str) -> Vec<Cue> {
    let mut cues = Vec::new();
    for block in text.replace("\r\n", "\n").replace('\r', "\n").split("\n\n") {
        let mut lines = block.lines().filter(|line| !line.trim().is_empty());
        let time_line = lines.find(|line| line.contains("-->"));
        let Some(time_line) = time_line else { continue };
        let Some((start, end)) = parse_range(time_line) else { continue };
        let spoken = lines.collect::<Vec<_>>().join(" ").trim().to_string();
        if spoken.is_empty() {
            continue;
        }
        cues.push(Cue { start, end, text: spoken });
    }
    cues
}

fn parse_range(line: &str) -> Option<(f64, f64)> {
    let (left, right) = line.split_once("-->")?;
    Some((parse_stamp(left.trim())?, parse_stamp(right.trim().split_whitespace().next()?)?))
}

fn parse_stamp(raw: &str) -> Option<f64> {
    let stamp = raw.trim().replace(',', ".");
    let mut parts = stamp.split(':').rev();
    let seconds = parts.next()?.parse::<f64>().ok()?;
    let minutes = parts.next().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
    let hours = parts.next().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
    Some(hours * 3600.0 + minutes * 60.0 + seconds)
}

pub fn plan_even(duration: f64, target: f64, count: usize) -> Vec<Clip> {
    if duration <= 1.0 || count == 0 {
        return Vec::new();
    }
    let width = target.clamp(8.0, duration);
    let slots = count.max(1);
    let mut clips = Vec::new();
    for index in 0..slots {
        let start = if slots == 1 {
            ((duration - width) / 2.0).max(0.0)
        } else {
            let span = (duration - width).max(0.0);
            span * (index as f64) / ((slots - 1) as f64)
        };
        let end = (start + width).min(duration);
        if end - start < 3.0 {
            continue;
        }
        clips.push(Clip {
            id: (index as u64) + 1,
            title: format!("Bagian {}", index + 1),
            start,
            end,
            score: (88u8).saturating_sub((index as u8) * 6),
            quote: String::new(),
            caption: String::new(),
            selected: true,
        });
    }
    clips
}

pub fn plan_from_cues(cues: &[Cue], target: f64, count: usize, duration: f64) -> Vec<Clip> {
    if cues.is_empty() || count == 0 {
        return plan_even(duration, target, count);
    }
    let mut clips = Vec::new();
    let mut cursor = 0usize;
    while clips.len() < count && cursor < cues.len() {
        let start = cues[cursor].start;
        let mut end = cues[cursor].end;
        let mut words = vec![cues[cursor].text.clone()];
        let mut next = cursor + 1;
        while next < cues.len() && (end - start) < target {
            end = cues[next].end;
            words.push(cues[next].text.clone());
            next += 1;
        }
        let end = end.min(duration.max(end));
        if end - start >= 3.0 {
            let quote = words.join(" ");
            let title = title_from_quote(&quote, clips.len() + 1);
            clips.push(Clip {
                id: (clips.len() as u64) + 1,
                title,
                start,
                end,
                score: (90u8).saturating_sub((clips.len() as u8) * 5),
                quote: quote.chars().take(220).collect(),
                caption: String::new(),
                selected: true,
            });
        }
        cursor = next.max(cursor + 1);
    }
    if clips.is_empty() {
        plan_even(duration, target, count)
    } else {
        clips
    }
}

fn title_from_quote(quote: &str, index: usize) -> String {
    let words: Vec<&str> = quote.split_whitespace().take(6).collect();
    if words.is_empty() {
        return format!("Bagian {index}");
    }
    let mut title = words.join(" ");
    let lower = title.to_lowercase();
    for banned in ["saya ", "aku ", "gue ", "i ", "i'm ", "my "] {
        if lower.starts_with(banned) {
            title = format!("Cuplikan {index}");
            break;
        }
    }
    title
}

pub fn analyze(
    source: &Source,
    cues: &[Cue],
    api_key: &str,
    model: &str,
    target: f64,
    count: usize,
    focus: &str,
    language: &str,
) -> Result<(String, Vec<Clip>), String> {
    let key = api_key.trim();
    if key.is_empty() || key.eq_ignore_ascii_case("mock") {
        let clips = if cues.is_empty() {
            plan_even(source.duration, target, count)
        } else {
            plan_from_cues(cues, target, count, source.duration)
        };
        let summary = if cues.is_empty() {
            "Tidak ada subtitle. Klip dibagi dari durasi video.".into()
        } else {
            "Klip disusun dari subtitle, tanpa panggilan model.".into()
        };
        return Ok((summary, clips));
    }
    match ask_gemini(source, cues, key, model, target, count, focus, language) {
        Ok(result) => Ok(result),
        Err(err) => {
            let clips = if cues.is_empty() {
                plan_even(source.duration, target, count)
            } else {
                plan_from_cues(cues, target, count, source.duration)
            };
            Ok((format!("Model gagal ({err}). Dipakai pembagian lokal."), clips))
        }
    }
}

fn ask_gemini(
    source: &Source,
    cues: &[Cue],
    api_key: &str,
    model: &str,
    target: f64,
    count: usize,
    focus: &str,
    language: &str,
) -> Result<(String, Vec<Clip>), String> {
    let lang = if language == "en" { "English" } else { "Indonesian" };
    let transcript = if cues.is_empty() {
        format!("No transcript. Duration {:.0} seconds. Invent standalone ranges inside the video.", source.duration)
    } else {
        cues.iter()
            .take(400)
            .map(|cue| format!("{:.2}|{:.2} {}", cue.start, cue.end, cue.text))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let focus_line = if focus.trim().is_empty() {
        String::new()
    } else {
        format!("User focus: {}. Keep the output language as {lang}.\n", focus.trim())
    };
    let prompt = format!(
        "You are a short-form editor. Pick {count} standalone clips from this video.\n\
         Title: {}\nDuration: {:.0}s. Each clip should be about {:.0} seconds, never under 8.\n\
         Write every title, summary, caption, and quote in {lang}. Do not translate away from {lang}.\n\
         Never use first person (I, me, my, saya, aku, gue) in titles.\n\
         {focus_line}\
         Transcript lines are start|end text:\n{transcript}\n\
         Return only JSON: {{\"summary\":\"\",\"clips\":[{{\"title\":\"\",\"start_time\":0,\"end_time\":0,\"virality_score\":80,\"key_quotes\":[\"\"],\"caption_suggestion\":\"\"}}]}}",
        source.title, source.duration, target, focus_line = focus_line, transcript = transcript
    );
    let body = serde_json::json!({
        "contents": [{"role": "user", "parts": [{"text": prompt}]}],
        "generationConfig": {"temperature": 0.4, "responseMimeType": "application/json"}
    });
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );
    let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(90)).build();
    let response = agent
        .post(&url)
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .map_err(|err| err.to_string())?;
    let raw_body = response.into_string().map_err(|err| err.to_string())?;
    let payload: serde_json::Value = serde_json::from_str(&raw_body).map_err(|err| err.to_string())?;
    let text = payload["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .ok_or_else(|| "Model tidak mengembalikan teks.".to_string())?;
    parse_model_json(text, source.duration)
}

pub fn parse_model_json(raw: &str, duration: f64) -> Result<(String, Vec<Clip>), String> {
    let mut clean = raw.trim().to_string();
    if clean.starts_with("```") {
        clean = clean.trim_start_matches("```json").trim_start_matches("```").trim().to_string();
        if let Some(stripped) = clean.strip_suffix("```") {
            clean = stripped.trim().to_string();
        }
    }
    let value: serde_json::Value = serde_json::from_str(&clean)
        .or_else(|_| {
            let start = clean.find('{').unwrap_or(0);
            let end = clean.rfind('}').map(|i| i + 1).unwrap_or(clean.len());
            serde_json::from_str(&clean[start..end])
        })
        .map_err(|err| format!("JSON model tidak terbaca: {err}"))?;
    let summary = value["summary"].as_str().unwrap_or("").to_string();
    let mut clips = Vec::new();
    let Some(list) = value["clips"].as_array() else {
        return Err("JSON model tidak berisi klip.".into());
    };
    for (index, item) in list.iter().enumerate() {
        let start = item["start_time"].as_f64().unwrap_or(0.0).max(0.0);
        let mut end = item["end_time"].as_f64().unwrap_or(start + 20.0);
        if duration > 0.0 {
            end = end.min(duration);
        }
        if end - start < 3.0 {
            continue;
        }
        let quote = item["key_quotes"]
            .as_array()
            .and_then(|quotes| quotes.first())
            .and_then(|quote| quote.as_str())
            .unwrap_or("")
            .to_string();
        clips.push(Clip {
            id: (index as u64) + 1,
            title: item["title"].as_str().unwrap_or("Tanpa judul").trim().to_string(),
            start,
            end,
            score: item["virality_score"].as_u64().unwrap_or(70).min(100) as u8,
            quote,
            caption: item["caption_suggestion"].as_str().unwrap_or("").to_string(),
            selected: true,
        });
    }
    if clips.is_empty() {
        return Err("Model tidak menghasilkan klip yang bisa dipakai.".into());
    }
    clips.sort_by(|a, b| b.score.cmp(&a.score));
    Ok((summary, clips))
}

pub fn export_clips(
    source: &Source,
    clips: &[Clip],
    output_dir: &Path,
    aspect: Aspect,
    burn_title: bool,
) -> Result<Vec<PathBuf>, String> {
    fs::create_dir_all(output_dir).map_err(|err| err.to_string())?;
    let input = PathBuf::from(&source.path);
    if !input.is_file() {
        return Err("File sumber sudah tidak ada.".into());
    }
    let mut written = Vec::new();
    for (index, clip) in clips.iter().filter(|clip| clip.selected).enumerate() {
        let name = format!("{:02}-{}.mp4", index + 1, slug(&clip.title));
        let dest = output_dir.join(name);
        export_one(&input, clip, &dest, aspect, burn_title)?;
        written.push(dest);
    }
    if written.is_empty() {
        return Err("Tidak ada klip yang ditandai.".into());
    }
    Ok(written)
}

fn export_one(input: &Path, clip: &Clip, dest: &Path, aspect: Aspect, burn_title: bool) -> Result<(), String> {
    let (width, height) = aspect.size();
    let mut filters = vec![
        format!("scale={width}:{height}:force_original_aspect_ratio=increase"),
        format!("crop={width}:{height}"),
    ];
    if burn_title {
        if let Some(font) = font_file() {
            let text = escape_drawtext(&clip.title);
            filters.push(format!(
                "drawtext=fontfile='{font}':text='{text}':fontsize=46:fontcolor=white:borderw=2:bordercolor=black@0.7:x=(w-text_w)/2:y=h*0.08"
            ));
        }
    }
    let start = format!("{:.3}", clip.start);
    let end = format!("{:.3}", clip.end);
    let filter = filters.join(",");
    let status = Command::new("ffmpeg")
        .args(["-y", "-ss", &start, "-to", &end, "-i"])
        .arg(input)
        .args(["-vf", &filter, "-c:v", "libx264", "-preset", "veryfast", "-crf", "20", "-c:a", "aac", "-b:a", "160k", "-movflags", "+faststart"])
        .arg(dest)
        .status()
        .map_err(|err| format!("ffmpeg: {err}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("ffmpeg gagal pada {}", clip.title))
    }
}

fn font_file() -> Option<&'static str> {
    const CANDIDATES: &[&str] = &[
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
        "/usr/share/fonts/truetype/freefont/FreeSansBold.ttf",
        "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
        "/Library/Fonts/Arial Bold.ttf",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "C:\\Windows\\Fonts\\arialbd.ttf",
        "C:\\Windows\\Fonts\\arial.ttf",
    ];
    CANDIDATES.iter().copied().find(|path| Path::new(path).is_file())
}

fn escape_drawtext(text: &str) -> String {
    text.replace('\\', "")
        .replace('\'', "")
        .replace(':', "\\:")
        .replace(',', "\\,")
        .replace('%', "")
        .chars()
        .take(72)
        .collect()
}

fn slug(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            for lower in ch.to_lowercase() {
                out.push(lower);
            }
        } else if ch.is_whitespace() || ch == '-' {
            if !out.ends_with('-') {
                out.push('-');
            }
        }
        if out.len() >= 42 {
            break;
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() { "klip".into() } else { out }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_srt_blocks() {
        let text = "1\n00:00:01,000 --> 00:00:03,500\nHalo semua\n\n2\n00:00:04,000 --> 00:00:06,000\nLanjut cerita\n";
        let cues = parse_srt(text);
        assert_eq!(cues.len(), 2);
        assert!((cues[0].start - 1.0).abs() < 0.01);
        assert_eq!(cues[1].text, "Lanjut cerita");
    }

    #[test]
    fn even_plan_stays_inside_duration() {
        let clips = plan_even(120.0, 30.0, 4);
        assert_eq!(clips.len(), 4);
        assert!(clips.iter().all(|clip| clip.end <= 120.0 && clip.end > clip.start));
    }

    #[test]
    fn cue_plan_groups_lines() {
        let cues = vec![
            Cue { start: 0.0, end: 4.0, text: "satu".into() },
            Cue { start: 4.0, end: 10.0, text: "dua".into() },
            Cue { start: 10.0, end: 16.0, text: "tiga".into() },
            Cue { start: 40.0, end: 48.0, text: "empat".into() },
        ];
        let clips = plan_from_cues(&cues, 12.0, 2, 60.0);
        assert_eq!(clips.len(), 2);
        assert!(clips[0].end - clips[0].start >= 12.0 || clips[0].end > 10.0);
    }

    #[test]
    fn slug_keeps_indonesian_letters() {
        assert_eq!(slug("Judul Cuplikan"), "judul-cuplikan");
        assert!(slug("Cerita singkat").starts_with("cerita"));
    }

    #[test]
    fn export_cuts_the_requested_span() {
        if !tool_exists("ffmpeg") || !tool_exists("ffprobe") {
            return;
        }
        let dir = std::env::temp_dir().join(format!("cheat-clip-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let src = dir.join("src.mp4");
        let status = Command::new("ffmpeg")
            .args([
                "-y",
                "-f", "lavfi", "-i", "testsrc=size=160x90:rate=25",
                "-f", "lavfi", "-i", "sine=frequency=440:sample_rate=44100",
                "-t", "6",
                "-c:v", "libx264", "-pix_fmt", "yuv420p",
                "-c:a", "aac",
                "-shortest",
            ])
            .arg(&src)
            .status()
            .expect("ffmpeg");
        assert!(status.success(), "failed to build the sample video");
        let source = Source {
            path: src.to_string_lossy().to_string(),
            title: "uji".into(),
            duration: 6.0,
            origin: "test".into(),
        };
        let clip = Clip {
            id: 1,
            title: "Cuplikan uji".into(),
            start: 1.0,
            end: 3.0,
            score: 80,
            quote: String::new(),
            caption: String::new(),
            selected: true,
        };
        let files = export_clips(&source, &[clip], &dir.join("out"), Aspect::Wide, false).unwrap();
        let dur = probe_duration(&files[0]).unwrap();
        assert!((dur - 2.0).abs() < 0.4, "duration was {dur}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn reads_model_json() {
        let raw = r#"{"summary":"ringkas","clips":[{"title":"Host menjelaskan","start_time":2,"end_time":20,"virality_score":91,"key_quotes":["kutipan"],"caption_suggestion":"caption"}]}"#;
        let (summary, clips) = parse_model_json(raw, 100.0).unwrap();
        assert_eq!(summary, "ringkas");
        assert_eq!(clips[0].score, 91);
        assert_eq!(clips[0].quote, "kutipan");
    }
}
