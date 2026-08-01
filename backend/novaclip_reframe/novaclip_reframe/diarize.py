#!/usr/bin/env python3
"""
Lightweight Offline Local Speaker Diarization Module for NovaClip
Extracts MFCC + spectral filterbank acoustic features per word timestamp
and clusters them into speaker IDs using Cosine Agglomerative Clustering.
"""

import sys
import json
import argparse
import os
import subprocess
import numpy as np
import scipy.io.wavfile as wavfile
from scipy.fftpack import dct
from sklearn.cluster import AgglomerativeClustering

def extract_wav(input_path, wav_path):
    cmd = [
        "ffmpeg", "-y", "-i", input_path,
        "-vn", "-ac", "1", "-ar", "16000", "-f", "wav", wav_path
    ]
    subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True)

def compute_mfcc(signal, sr=16000, n_mfcc=13, nfilt=26, nfft=512):
    pre_emphasis = 0.97
    emphasized_signal = np.append(signal[0], signal[1:] - pre_emphasis * signal[:-1])
    frame_size, frame_stride = 0.025, 0.010
    frame_length, frame_step = frame_size * sr, frame_stride * sr
    signal_length = len(emphasized_signal)
    frame_length = int(round(frame_length))
    frame_step = int(round(frame_step))
    num_frames = int(np.ceil(float(np.abs(signal_length - frame_length)) / frame_step))

    if num_frames <= 0:
        return np.zeros((1, n_mfcc))

    pad_signal_length = num_frames * frame_step + frame_length
    z = np.zeros((pad_signal_length - signal_length))
    pad_signal = np.append(emphasized_signal, z)

    indices = np.tile(np.arange(0, frame_length), (num_frames, 1)) + np.tile(np.arange(0, num_frames * frame_step, frame_step), (frame_length, 1)).T
    frames = pad_signal[indices.astype(np.int32, copy=False)]
    frames *= np.hamming(frame_length)

    mag_frames = np.absolute(np.fft.rfft(frames, nfft))
    pow_frames = ((1.0 / nfft) * ((mag_frames) ** 2))

    low_freq_mel = 0
    high_freq_mel = (2595 * np.log10(1 + (sr / 2) / 700))
    mel_points = np.linspace(low_freq_mel, high_freq_mel, nfilt + 2)
    hz_points = (700 * (10**(mel_points / 2595) - 1))
    bin = np.floor((nfft + 1) * hz_points / sr)

    fbank = np.zeros((nfilt, int(np.floor(nfft / 2 + 1))))
    for m in range(1, nfilt + 1):
        f_m_minus = int(bin[m - 1])
        f_m = int(bin[m])
        f_m_plus = int(bin[m + 1])

        for k in range(f_m_minus, f_m):
            fbank[m - 1, k] = (k - bin[m - 1]) / (bin[m] - bin[m - 1])
        for k in range(f_m, f_m_plus):
            fbank[m - 1, k] = (bin[m + 1] - k) / (bin[m + 1] - bin[m])

    filter_banks = np.dot(pow_frames, fbank.T)
    filter_banks = np.where(filter_banks == 0, np.finfo(float).eps, filter_banks)
    filter_banks = 20 * np.log10(filter_banks)

    mfcc = dct(filter_banks, type=2, axis=1, norm='ortho')[:, 1 : (n_mfcc + 1)]
    return mfcc

def extract_word_embedding(signal, sr, start_sec, end_sec):
    pad = 0.05
    s = max(0, int((start_sec - pad) * sr))
    e = min(len(signal), int((end_sec + pad) * sr))

    if e - s < int(0.15 * sr):
        center = (s + e) // 2
        s = max(0, center - int(0.1 * sr))
        e = min(len(signal), center + int(0.1 * sr))

    segment = signal[s:e]
    if len(segment) < 200:
        return np.zeros(26)

    mfcc = compute_mfcc(segment, sr=sr)
    if len(mfcc) == 0:
        return np.zeros(26)

    mean = np.mean(mfcc, axis=0)
    std = np.std(mfcc, axis=0)
    return np.concatenate([mean, std])

def smooth_speaker_labels(words, min_hold_words=2):
    """Smooths out single-word noise toggles within continuous speech."""
    if len(words) < 3:
        return words

    labels = [w.get("speaker", 0) for w in words]
    n = len(labels)

    for i in range(1, n - 1):
        prev_lbl = labels[i - 1]
        curr_lbl = labels[i]
        next_lbl = labels[i + 1]

        # Single word isolated flip
        if curr_lbl != prev_lbl and prev_lbl == next_lbl:
            gap_prev = words[i]["start"] - words[i - 1]["end"]
            gap_next = words[i + 1]["start"] - words[i]["end"]
            if gap_prev < 0.4 and gap_next < 0.4:
                labels[i] = prev_lbl

    for i, w in enumerate(words):
        w["speaker"] = labels[i]

    return words

def diarize_transcript(audio_file, words, n_speakers=2):
    if not words or len(words) == 0:
        return words

    tmp_wav = audio_file + ".diarize_tmp.wav"
    try:
        extract_wav(audio_file, tmp_wav)
        sr, signal = wavfile.read(tmp_wav)
        if signal.ndim > 1:
            signal = signal.mean(axis=1)

        embeddings = []
        valid_indices = []

        for idx, w in enumerate(words):
            start = float(w.get("start", 0.0))
            end = float(w.get("end", 0.0))
            emb = extract_word_embedding(signal, sr, start, end)
            if not np.all(emb == 0):
                embeddings.append(emb)
                valid_indices.append(idx)

        if len(embeddings) < n_speakers:
            for w in words:
                w["speaker"] = 0
            return words

        embeddings = np.array(embeddings)
        clustering = AgglomerativeClustering(n_clusters=n_speakers, metric="cosine", linkage="average")
        labels = clustering.fit_predict(embeddings)

        for idx, label in zip(valid_indices, labels):
            words[idx]["speaker"] = int(label)

        last_spk = 0
        for w in words:
            if "speaker" not in w:
                w["speaker"] = last_spk
            else:
                last_spk = w["speaker"]

        return smooth_speaker_labels(words)
    except Exception as e:
        sys.stderr.write(f"Diarization warning: {e}\n")
        for w in words:
            if "speaker" not in w:
                w["speaker"] = 0
        return words
    finally:
        if os.path.exists(tmp_wav):
            try:
                os.remove(tmp_wav)
            except Exception:
                pass

def main():
    parser = argparse.ArgumentParser(description="NovaClip Local Diarization Engine")
    parser.add_argument("--audio", required=True, help="Path to audio or video file")
    parser.add_argument("--num-speakers", type=int, default=2, help="Number of speakers")
    args = parser.parse_args()

    input_json = sys.stdin.read()
    if not input_json.strip():
        sys.stderr.write("Error: empty input json on stdin\n")
        sys.exit(1)

    words = json.loads(input_json)
    diarized = diarize_transcript(args.audio, words, n_speakers=args.num_speakers)
    print(json.dumps(diarized))

if __name__ == "__main__":
    main()
