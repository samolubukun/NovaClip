#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import logging
import math
import shutil
import subprocess
import tempfile
from collections import deque
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Optional

import cv2
import numpy as np
from scenedetect import AdaptiveDetector, SceneManager, open_video
from ultralytics import YOLO

try:
    from mediapipe.python.solutions import face_detection as mp_face_detection
except Exception:
    try:
        from mediapipe import solutions as mp_solutions
        mp_face_detection = mp_solutions.face_detection
    except Exception:
        mp_face_detection = None

try:
    from mediapipe.python.solutions import pose as mp_pose
except Exception:
    try:
        from mediapipe import solutions as _mp_solutions_pose
        mp_pose = _mp_solutions_pose.pose
    except Exception:
        mp_pose = None

try:
    import torch
except Exception:
    torch = None


CLASS_IDS = {
    "person": 0,
    "bicycle": 1,
    "car": 2,
    "motorcycle": 3,
    "bus": 5,
    "truck": 7,
    "cat": 15,
    "dog": 16,
}

PRESETS = {
    "talking_head": {
        "classes": ["person"],
        "min_zoom": 1.05,
        "max_zoom": 1.85,
        "max_step_x": 7.0,
        "max_step_y": 5.0,
    },
    "sports": {
        "classes": ["person", "car", "bicycle", "motorcycle"],
        "min_zoom": 1.00,
        "max_zoom": 1.35,
        "max_step_x": 12.0,
        "max_step_y": 8.0,
    },
    "pets": {
        "classes": ["dog", "cat", "person"],
        "min_zoom": 1.00,
        "max_zoom": 1.55,
        "max_step_x": 10.0,
        "max_step_y": 7.0,
    },
    "cars": {
        "classes": ["car", "truck", "bus", "motorcycle", "person"],
        "min_zoom": 1.00,
        "max_zoom": 1.30,
        "max_step_x": 11.0,
        "max_step_y": 7.0,
    },
}


@dataclass
class Candidate:
    cls_id: int
    cls_name: str
    track_id: Optional[int]
    conf: float
    x1: float
    y1: float
    x2: float
    y2: float
    cx: float
    cy: float
    width: float
    height: float
    area: float
    mask_area: float
    mask_cx: float
    mask_cy: float
    mask_top_y: float
    framing_cx: float
    framing_cy: float
    face_box: Optional[tuple[int, int, int, int]]
    score: float = 0.0
    eye_y: Optional[float] = None
    chin_y: Optional[float] = None
    body_top_y: Optional[float] = None
    body_bottom_y: Optional[float] = None
    body_cx: Optional[float] = None
    shoulder_span: Optional[float] = None
    body_bottom_confident: bool = False
    body_min_x: Optional[float] = None
    body_max_x: Optional[float] = None
    salient_x1: Optional[float] = None
    salient_y1: Optional[float] = None
    salient_x2: Optional[float] = None
    salient_y2: Optional[float] = None
    saliency_confidence: float = 0.0


@dataclass
class CameraObservation:
    center_x: float
    center_y: float
    zoom: float
    confidence: float


@dataclass
class CameraState:
    crop_center_x: float
    crop_center_y: float
    zoom: float
    target_center_x: float
    target_center_y: float
    target_zoom: float
    velocity_x: float = 0.0
    velocity_y: float = 0.0
    zoom_velocity: float = 0.0
    tracked_id: Optional[int] = None
    tracked_cls_id: Optional[int] = None
    missed_frames: int = 0
    lock_track_id: Optional[int] = None
    lock_cls_id: Optional[int] = None
    current_scene_index: int = 0
    frames_since_subject_switch: int = 0
    last_framing_cx: Optional[float] = None
    last_framing_cy: Optional[float] = None
    last_subject_key: Optional[tuple[Optional[int], int]] = None
    framing_vx: float = 0.0
    framing_vy: float = 0.0


class HandcraftedSaliencyHelper:
    def __init__(self) -> None:
        self.prev_gray_small: Optional[np.ndarray] = None
        self.backend_name = "handcrafted"
        self.active_backend = "handcrafted"
        self.frames_total = 0
        self.frames_backend = 0
        self.frames_fallback = 0

    def compute_map(self, frame_bgr: np.ndarray) -> np.ndarray:
        self.frames_total += 1
        self.frames_backend += 1
        frame_h, frame_w = frame_bgr.shape[:2]
        scale = min(1.0, 320.0 / max(frame_h, frame_w))
        small_w = max(32, int(round(frame_w * scale)))
        small_h = max(32, int(round(frame_h * scale)))

        gray_small = cv2.cvtColor(
            cv2.resize(
                frame_bgr,
                (small_w, small_h),
                interpolation=cv2.INTER_AREA,
            ),
            cv2.COLOR_BGR2GRAY,
        ).astype(np.float32)

        dft = cv2.dft(gray_small, flags=cv2.DFT_COMPLEX_OUTPUT)
        real = dft[:, :, 0]
        imag = dft[:, :, 1]
        magnitude = cv2.magnitude(real, imag)
        log_amplitude = np.log(magnitude + 1e-6)
        spectral_residual = log_amplitude - cv2.blur(log_amplitude, (3, 3))
        phase = cv2.phase(real, imag)

        exp_residual = np.exp(spectral_residual)
        residual_real = exp_residual * np.cos(phase)
        residual_imag = exp_residual * np.sin(phase)
        residual_spectrum = np.dstack([residual_real, residual_imag]).astype(
            np.float32
        )
        saliency_small = cv2.idft(
            residual_spectrum,
            flags=cv2.DFT_SCALE | cv2.DFT_REAL_OUTPUT,
        )
        saliency_small = cv2.GaussianBlur(
            saliency_small * saliency_small,
            (7, 7),
            0,
        )

        saliency_small = cv2.normalize(
            saliency_small, None, 0.0, 1.0, cv2.NORM_MINMAX
        )

        if (
            self.prev_gray_small is not None
            and self.prev_gray_small.shape == gray_small.shape
        ):
            motion = cv2.absdiff(gray_small, self.prev_gray_small)
            motion = cv2.GaussianBlur(motion, (5, 5), 0)
            motion = cv2.normalize(motion, None, 0.0, 1.0, cv2.NORM_MINMAX)
            saliency_small = saliency_small * 0.72 + motion * 0.28

        self.prev_gray_small = gray_small

        return cv2.resize(
            saliency_small,
            (frame_w, frame_h),
            interpolation=cv2.INTER_LINEAR,
        )

    def reset_temporal_state(self) -> None:
        self.prev_gray_small = None

    def get_telemetry(self) -> dict[str, Any]:
        return {
            "requested_backend": self.backend_name,
            "active_backend": self.active_backend,
            "frames_total": self.frames_total,
            "frames_backend": self.frames_backend,
            "frames_fallback": self.frames_fallback,
            "model_loaded": False,
        }


class DeepGazeMRSaliencyHelper:
    def __init__(
        self,
        device: str = "auto",
        max_side: int = 384,
    ) -> None:
        self.backend_name = "deepgazemr"
        self.device_name = self._resolve_device(device)
        self.max_side = max(128, int(max_side))
        self.model = None
        self.frame_buffer: deque[np.ndarray] = deque(maxlen=16)
        self.fallback = HandcraftedSaliencyHelper()
        self._disabled = False
        self.active_backend = "handcrafted"
        self.frames_total = 0
        self.frames_backend = 0
        self.frames_fallback = 0
        self.model_loaded = False

    def _resolve_device(self, device: str) -> str:
        if device != "auto":
            return device
        if torch is None:
            return "cpu"
        if torch.cuda.is_available():
            return "cuda"
        return "cpu"

    def _load_model(self) -> bool:
        if self._disabled:
            return False
        if self.model is not None:
            return True
        if torch is None:
            logging.warning(
                "PyTorch is unavailable; falling back to handcrafted saliency."
            )
            self._disabled = True
            return False
        try:
            self.model = torch.hub.load(
                "mtangemann/deepgazemr",
                "DeepGazeMR",
                pretrained=False,
                trust_repo=True,
            )
            repo_dir = (
                Path(torch.hub.get_dir()) / "mtangemann_deepgazemr_master"
            )
            checkpoint = torch.load(
                repo_dir / "data" / "deepgazemr-ledov.pt",
                map_location="cpu",
            )
            self.model.load_state_dict(checkpoint["model_state_dict"])
            center_bias = torch.load(
                repo_dir / "data" / "center-bias-ledov.pt",
                map_location="cpu",
            )
            if hasattr(self.model, "center_bias"):
                self.model.center_bias = center_bias
            self.model.to(self.device_name)
            if hasattr(self.model, "center_bias") and torch.is_tensor(
                self.model.center_bias
            ):
                self.model.center_bias = self.model.center_bias.to(
                    self.device_name
                )
            self.model.eval()
            self.active_backend = "deepgazemr"
            self.model_loaded = True
            logging.info(
                "Loaded DeepGaze MR saliency model on %s.",
                self.device_name,
            )
            return True
        except Exception as exc:
            logging.warning(
                "Failed to load DeepGaze MR (%s); falling back to handcrafted saliency.",
                exc,
            )
            self._disabled = True
            self.model = None
            self.active_backend = "handcrafted"
            return False

    def _preprocess_frame(
        self,
        frame_bgr: np.ndarray,
    ) -> tuple[np.ndarray, tuple[int, int]]:
        frame_h, frame_w = frame_bgr.shape[:2]
        scale = min(1.0, self.max_side / max(frame_h, frame_w))
        resized_w = max(64, int(round(frame_w * scale)))
        resized_h = max(64, int(round(frame_h * scale)))
        resized = cv2.resize(
            frame_bgr,
            (resized_w, resized_h),
            interpolation=cv2.INTER_AREA,
        )
        rgb = cv2.cvtColor(resized, cv2.COLOR_BGR2RGB).astype(np.float32) / 255.0
        chw = np.transpose(rgb, (2, 0, 1))
        return chw, (frame_w, frame_h)

    def compute_map(self, frame_bgr: np.ndarray) -> np.ndarray:
        self.frames_total += 1
        if not self._load_model():
            self.frames_fallback += 1
            return self.fallback.compute_map(frame_bgr)

        frame_chw, original_size = self._preprocess_frame(frame_bgr)
        self.frame_buffer.append(frame_chw)
        if len(self.frame_buffer) < 16:
            self.frames_fallback += 1
            return self.fallback.compute_map(frame_bgr)

        clip_np = np.stack(self.frame_buffer, axis=0)
        clip = torch.from_numpy(clip_np).to(self.device_name)

        try:
            with torch.no_grad():
                prediction = self.model.forward(clip)
            if prediction is None:
                self.frames_fallback += 1
                return self.fallback.compute_map(frame_bgr)

            self.frames_backend += 1
            saliency = prediction.detach().to("cpu").float().numpy()
            saliency = cv2.normalize(
                saliency,
                None,
                0.0,
                1.0,
                cv2.NORM_MINMAX,
            )
            return cv2.resize(
                saliency,
                original_size,
                interpolation=cv2.INTER_LINEAR,
            )
        except Exception as exc:
            logging.warning(
                "DeepGaze MR inference failed (%s); using handcrafted saliency for this frame.",
                exc,
            )
            self.frames_fallback += 1
            return self.fallback.compute_map(frame_bgr)

    def reset_temporal_state(self) -> None:
        self.frame_buffer.clear()
        self.fallback.reset_temporal_state()

    def get_telemetry(self) -> dict[str, Any]:
        return {
            "requested_backend": self.backend_name,
            "active_backend": self.active_backend,
            "frames_total": self.frames_total,
            "frames_backend": self.frames_backend,
            "frames_fallback": self.frames_fallback,
            "model_loaded": self.model_loaded,
            "device": self.device_name,
        }


def build_saliency_helper(args: argparse.Namespace) -> Any:
    if args.saliency_model == "handcrafted":
        return HandcraftedSaliencyHelper()
    if args.saliency_model == "deepgazemr":
        return DeepGazeMRSaliencyHelper(
            device=args.saliency_device,
            max_side=args.saliency_max_side,
        )
    helper = DeepGazeMRSaliencyHelper(
        device=args.saliency_device,
        max_side=args.saliency_max_side,
    )
    return helper


class SubjectRankingModel:
    def __init__(self) -> None:
        self.class_bias = {
            "person": 0.22,
            "dog": 0.12,
            "cat": 0.10,
            "car": 0.06,
            "bicycle": 0.02,
            "motorcycle": 0.02,
            "bus": 0.01,
            "truck": 0.01,
        }
        self.feature_weights = {
            "det_conf": 1.35,
            "mask_presence": 0.95,
            "center_affinity": 0.55,
            "face_presence": 0.48,
            "pose_presence": 0.34,
            "saliency_presence": 0.72,
            "saliency_conf": 0.78,
            "tracking_match": 1.05,
            "lock_match": 1.30,
            "speaker_active": 0.22,
            "size_logit": 0.26,
        }

    def predict(
        self,
        *,
        cls_name: str,
        conf: float,
        mask_area: float,
        frame_area: float,
        dist_center: float,
        frame_diag: float,
        has_face: bool,
        has_pose: bool,
        saliency_confidence: float,
        tracking_match: bool,
        lock_match: bool,
        speaker_active: bool,
    ) -> float:
        norm_area = clamp(mask_area / max(frame_area, 1.0), 0.0, 1.0)
        center_affinity = 1.0 - clamp(dist_center / max(frame_diag, 1.0), 0.0, 1.0)
        size_logit = math.log1p(norm_area * 250.0)

        features = {
            "det_conf": clamp(conf, 0.0, 1.0),
            "mask_presence": math.sqrt(norm_area),
            "center_affinity": center_affinity,
            "face_presence": 1.0 if has_face else 0.0,
            "pose_presence": 1.0 if has_pose else 0.0,
            "saliency_presence": 1.0 if saliency_confidence > 0.0 else 0.0,
            "saliency_conf": clamp(saliency_confidence, 0.0, 1.0),
            "tracking_match": 1.0 if tracking_match else 0.0,
            "lock_match": 1.0 if lock_match else 0.0,
            "speaker_active": 1.0 if speaker_active else 0.0,
            "size_logit": size_logit,
        }

        score = self.class_bias.get(cls_name, 0.0)
        for feature_name, feature_value in features.items():
            score += self.feature_weights[feature_name] * feature_value

        return score


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Scene-aware vertical auto-reframe with segmentation, presets, "
            "and two-person framing."
        )
    )
    parser.add_argument("input")
    parser.add_argument("output")

    parser.add_argument("--seg-model", default="yolo11n-seg.pt")
    parser.add_argument("--tracker", default="bytetrack.yaml")
    parser.add_argument("--conf", type=float, default=0.30)

    parser.add_argument(
        "--preset",
        choices=list(PRESETS.keys()),
        default="talking_head",
    )
    parser.add_argument("--classes", nargs="+", default=None)

    parser.add_argument("--output-width", type=int, default=1080)
    parser.add_argument("--output-height", type=int, default=1920)

    parser.add_argument("--zoom-alpha", type=float, default=0.035)
    parser.add_argument("--target-alpha", type=float, default=0.12)
    parser.add_argument("--motion-response", type=float, default=0.12)
    parser.add_argument("--motion-damping", type=float, default=0.82)
    parser.add_argument("--zoom-response", type=float, default=0.08)
    parser.add_argument("--zoom-damping", type=float, default=0.85)
    parser.add_argument("--max-missed-frames", type=int, default=40)
    parser.add_argument("--min-subject-hold-frames", type=int, default=12)
    parser.add_argument("--switch-score-threshold", type=float, default=1.20)

    parser.add_argument("--max-step-x", type=float, default=None)
    parser.add_argument("--max-step-y", type=float, default=None)

    parser.add_argument("--min-zoom", type=float, default=None)
    parser.add_argument("--max-zoom", type=float, default=None)

    parser.add_argument("--lock-first-subject", action="store_true")

    parser.add_argument("--two-person-framing", action="store_true")
    parser.add_argument("--two-person-threshold", type=float, default=0.78)

    parser.add_argument(
        "--layout",
        choices=["single", "split", "auto"],
        default="single",
        help=(
            "Camera layout: 'single' tracks one subject, 'split' renders two "
            "side-by-side panes each tracking one person, 'auto' chooses per "
            "frame based on whether two people are present."
        ),
    )
    parser.add_argument(
        "--split-divider",
        action="store_true",
        help="Draw a thin black divider line between split panes.",
    )

    parser.add_argument(
        "--saliency-model",
        choices=["auto", "deepgazemr", "handcrafted"],
        default="handcrafted",
    )
    parser.add_argument(
        "--saliency-device",
        choices=["auto", "cpu", "cuda", "mps"],
        default="auto",
    )
    parser.add_argument("--saliency-max-side", type=int, default=384)

    parser.add_argument("--speaker-aware-mode", action="store_true")
    parser.add_argument(
        "--speaker-json",
        default="",
        help=(
            "Optional external diarization JSON for future speaker-aware "
            "reranking"
        ),
    )

    parser.add_argument("--scene-threshold", type=float, default=3.0)
    parser.add_argument("--min-scene-len", type=int, default=15)

    parser.add_argument(
        "--post-restore",
        action="store_true",
        help="Apply light unsharp/denoise during final ffmpeg stage",
    )

    parser.add_argument("--video-encoder", default="h264_videotoolbox")
    parser.add_argument("--audio-bitrate", default="192k")
    parser.add_argument("--crf", type=int, default=18)
    parser.add_argument("--preset-ffmpeg", default="medium")

    parser.add_argument("--save-debug-preview", action="store_true")
    parser.add_argument("--debug-path", default="")
    parser.add_argument(
        "--log-level",
        default="INFO",
        choices=["DEBUG", "INFO", "WARNING", "ERROR"],
    )
    return parser.parse_args()


def setup_logging(level: str) -> None:
    logging.basicConfig(
        level=getattr(logging, level),
        format="%(asctime)s | %(levelname)s | %(message)s",
    )


def apply_preset(args: argparse.Namespace) -> argparse.Namespace:
    preset = PRESETS[args.preset]

    if args.classes is None:
        args.classes = preset["classes"]

    for key in [
        "max_step_x",
        "max_step_y",
        "min_zoom",
        "max_zoom",
    ]:
        if getattr(args, key) is None:
            setattr(args, key, preset[key])

    return args


def clamp(value: float, low: float, high: float) -> float:
    return max(low, min(value, high))


def lerp(a: float, b: float, alpha: float) -> float:
    return a + (b - a) * alpha


def limit_step(current: float, target: float, max_step: float) -> float:
    delta = target - current
    if delta > max_step:
        return current + max_step
    if delta < -max_step:
        return current - max_step
    return target


def compute_base_crop(
    frame_w: int,
    frame_h: int,
    aspect_w: int = 9,
    aspect_h: int = 16,
) -> tuple[int, int]:
    target_ratio = aspect_w / aspect_h
    frame_ratio = frame_w / frame_h
    if frame_ratio >= target_ratio:
        crop_h = frame_h
        crop_w = int(round(crop_h * target_ratio))
    else:
        crop_w = frame_w
        crop_h = int(round(crop_w / target_ratio))
    return min(crop_w, frame_w), min(crop_h, frame_h)


def current_crop_size(
    base_crop_w: int,
    base_crop_h: int,
    zoom: float,
    frame_w: int,
    frame_h: int,
) -> tuple[int, int]:
    crop_w = int(round(base_crop_w / zoom))
    crop_h = int(round(base_crop_h / zoom))
    crop_w = max(64, min(crop_w, frame_w))
    crop_h = max(64, min(crop_h, frame_h))
    return crop_w, crop_h


def get_track_id(box: Any) -> Optional[int]:
    try:
        if box.id is None:
            return None
        return int(box.id[0].item())
    except Exception:
        return None


class MediaPipeFaceHelper:
    def __init__(self, min_detection_confidence: float = 0.45):
        self.detector = None
        if mp_face_detection is None:
            logging.warning(
                "MediaPipe face detection is unavailable in this environment; "
                "continuing without face-priority framing."
            )
            return

        try:
            self.detector = mp_face_detection.FaceDetection(
                model_selection=0,
                min_detection_confidence=min_detection_confidence,
            )
        except Exception as exc:
            logging.warning(
                "MediaPipe face detection could not be initialized: %s. "
                "Continuing without face-priority framing.",
                exc,
            )

    def detect_in_person_box(
        self,
        frame_bgr: np.ndarray,
        person_box: tuple[int, int, int, int],
    ) -> Optional[tuple[int, int, int, int]]:
        if self.detector is None:
            return None

        h, w = frame_bgr.shape[:2]
        x1, y1, x2, y2 = person_box
        x1 = int(clamp(x1, 0, w - 1))
        y1 = int(clamp(y1, 0, h - 1))
        x2 = int(clamp(x2, x1 + 1, w))
        y2 = int(clamp(y2, y1 + 1, h))

        roi = frame_bgr[y1:y2, x1:x2]
        if roi.size == 0:
            return None

        upper_h = max(1, int((y2 - y1) * 0.65))
        roi = roi[:upper_h, :]
        rgb = cv2.cvtColor(roi, cv2.COLOR_BGR2RGB)
        result = self.detector.process(rgb)
        if not result.detections:
            return None

        best = None
        best_area = -1.0
        for det in result.detections:
            bbox = det.location_data.relative_bounding_box
            fx1 = max(0.0, bbox.xmin)
            fy1 = max(0.0, bbox.ymin)
            fw = max(0.0, bbox.width)
            fh = max(0.0, bbox.height)

            px1 = int(round(fx1 * roi.shape[1]))
            py1 = int(round(fy1 * roi.shape[0]))
            px2 = int(round((fx1 + fw) * roi.shape[1]))
            py2 = int(round((fy1 + fh) * roi.shape[0]))

            area = max(1, px2 - px1) * max(1, py2 - py1)
            if area > best_area:
                best_area = area
                best = (x1 + px1, y1 + py1, x1 + px2, y1 + py2)
        return best


POSE_LANDMARK_NAMES = {
    0: "nose",
    2: "left_eye",
    5: "right_eye",
    7: "left_ear",
    8: "right_ear",
    9: "mouth_left",
    10: "mouth_right",
    11: "left_shoulder",
    12: "right_shoulder",
    13: "left_elbow",
    14: "right_elbow",
    15: "left_wrist",
    16: "right_wrist",
    23: "left_hip",
    24: "right_hip",
    25: "left_knee",
    26: "right_knee",
    27: "left_ankle",
    28: "right_ankle",
    29: "left_heel",
    30: "right_heel",
    31: "left_foot_index",
    32: "right_foot_index",
}


class MediaPipePoseHelper:
    def __init__(self, min_detection_confidence: float = 0.35):
        self.detector = None
        self.min_visibility = 0.35
        if mp_pose is None:
            logging.warning(
                "MediaPipe pose is unavailable; composition will fall back "
                "to mask-based framing."
            )
            return

        try:
            self.detector = mp_pose.Pose(
                static_image_mode=False,
                model_complexity=1,
                enable_segmentation=False,
                min_detection_confidence=min_detection_confidence,
                min_tracking_confidence=0.4,
            )
        except Exception as exc:
            logging.warning(
                "MediaPipe pose could not be initialized: %s. "
                "Continuing without pose-driven framing.",
                exc,
            )

    def detect_in_person_box(
        self,
        frame_bgr: np.ndarray,
        person_box: tuple[int, int, int, int],
        pad_ratio: float = 0.12,
    ) -> Optional[dict]:
        if self.detector is None:
            return None

        h, w = frame_bgr.shape[:2]
        x1, y1, x2, y2 = person_box
        bw = max(1, x2 - x1)
        bh = max(1, y2 - y1)
        pad_x = int(bw * pad_ratio)
        pad_y = int(bh * pad_ratio)
        rx1 = int(clamp(x1 - pad_x, 0, w - 1))
        ry1 = int(clamp(y1 - pad_y, 0, h - 1))
        rx2 = int(clamp(x2 + pad_x, rx1 + 1, w))
        ry2 = int(clamp(y2 + pad_y, ry1 + 1, h))

        roi = frame_bgr[ry1:ry2, rx1:rx2]
        if roi.size == 0:
            return None

        try:
            rgb = cv2.cvtColor(roi, cv2.COLOR_BGR2RGB)
            result = self.detector.process(rgb)
        except Exception:
            return None

        if not result.pose_landmarks:
            return None

        roi_h, roi_w = roi.shape[:2]
        points: dict[str, tuple[float, float, float]] = {}
        for idx, lm in enumerate(result.pose_landmarks.landmark):
            name = POSE_LANDMARK_NAMES.get(idx)
            if name is None:
                continue
            px = rx1 + lm.x * roi_w
            py = ry1 + lm.y * roi_h
            vis = float(getattr(lm, "visibility", 0.0))
            points[name] = (px, py, vis)

        if not points:
            return None

        def visible(name: str) -> Optional[tuple[float, float]]:
            p = points.get(name)
            if p is None:
                return None
            if p[2] < self.min_visibility:
                return None
            return p[0], p[1]

        left_eye = visible("left_eye")
        right_eye = visible("right_eye")
        nose = visible("nose")
        if left_eye and right_eye:
            eye_y = (left_eye[1] + right_eye[1]) / 2
            eye_cx = (left_eye[0] + right_eye[0]) / 2
        elif nose is not None:
            eye_y = nose[1]
            eye_cx = nose[0]
        else:
            eye_y = None
            eye_cx = None

        mouth_l = visible("mouth_left")
        mouth_r = visible("mouth_right")
        if mouth_l and mouth_r:
            chin_y = max(mouth_l[1], mouth_r[1])
        else:
            chin_y = None

        left_sh = visible("left_shoulder")
        right_sh = visible("right_shoulder")
        if left_sh and right_sh:
            shoulder_span = abs(left_sh[0] - right_sh[0])
            shoulder_cx = (left_sh[0] + right_sh[0]) / 2
            shoulder_y = (left_sh[1] + right_sh[1]) / 2
        else:
            shoulder_span = None
            shoulder_cx = None
            shoulder_y = None

        left_hip = visible("left_hip")
        right_hip = visible("right_hip")
        if left_hip and right_hip:
            hip_cx = (left_hip[0] + right_hip[0]) / 2
            hip_y = (left_hip[1] + right_hip[1]) / 2
        else:
            hip_cx = None
            hip_y = None

        lower_body_names = [
            "left_knee",
            "right_knee",
            "left_ankle",
            "right_ankle",
            "left_heel",
            "right_heel",
            "left_foot_index",
            "right_foot_index",
        ]
        lower_body_visible = [
            visible(name) for name in lower_body_names
        ]
        lower_body_visible = [p for p in lower_body_visible if p is not None]

        visible_ys = [
            p[1] for p in points.values() if p[2] >= self.min_visibility
        ]
        visible_xs = [
            p[0] for p in points.values() if p[2] >= self.min_visibility
        ]
        if not visible_ys or not visible_xs:
            return None

        body_top_candidates = []
        if eye_y is not None:
            body_top_candidates.append(eye_y)
        else:
            body_top_candidates.append(min(visible_ys))

        ear_y_values = [
            points[n][1]
            for n in ("left_ear", "right_ear")
            if n in points and points[n][2] >= self.min_visibility
        ]
        if ear_y_values:
            body_top_candidates.append(min(ear_y_values))

        body_top_y = min(body_top_candidates)

        body_bottom_y = max(visible_ys)
        body_bottom_confident = len(lower_body_visible) > 0

        if shoulder_cx is not None and hip_cx is not None:
            body_cx = (shoulder_cx + hip_cx) / 2
        elif shoulder_cx is not None:
            body_cx = shoulder_cx
        elif hip_cx is not None:
            body_cx = hip_cx
        elif eye_cx is not None:
            body_cx = eye_cx
        else:
            body_cx = None

        body_min_x = min(visible_xs)
        body_max_x = max(visible_xs)

        return {
            "eye_y": eye_y,
            "eye_cx": eye_cx,
            "chin_y": chin_y,
            "shoulder_y": shoulder_y,
            "shoulder_span": shoulder_span,
            "hip_y": hip_y,
            "body_top_y": body_top_y,
            "body_bottom_y": body_bottom_y,
            "body_bottom_confident": body_bottom_confident,
            "body_cx": body_cx,
            "body_min_x": body_min_x,
            "body_max_x": body_max_x,
        }


def detect_scenes(
    video_path: str,
    threshold: float,
    min_scene_len: int,
) -> list[int]:
    video = open_video(video_path)
    scene_manager = SceneManager()
    scene_manager.add_detector(
        AdaptiveDetector(
            adaptive_threshold=threshold,
            min_scene_len=min_scene_len,
        )
    )
    scene_manager.detect_scenes(video)
    scene_list = scene_manager.get_scene_list()

    starts = [1]
    for start_time, _ in scene_list:
        start_frame = start_time.get_frames() + 1
        starts.append(start_frame)
    return sorted(set(starts))


def mask_stats_from_binary_mask(
    mask: np.ndarray,
) -> tuple[float, float, float, float]:
    ys, xs = np.where(mask > 0)
    if len(xs) == 0:
        return 0.0, 0.0, 0.0, 0.0
    return float(len(xs)), float(xs.mean()), float(ys.mean()), float(ys.min())


def extract_saliency_region(
    saliency_map: np.ndarray,
    bounds: tuple[int, int, int, int],
) -> Optional[tuple[float, float, float, float, float]]:
    h, w = saliency_map.shape[:2]
    x1, y1, x2, y2 = bounds
    x1 = int(clamp(x1, 0, w - 1))
    y1 = int(clamp(y1, 0, h - 1))
    x2 = int(clamp(x2, x1 + 1, w))
    y2 = int(clamp(y2, y1 + 1, h))

    roi = saliency_map[y1:y2, x1:x2]
    if roi.size == 0:
        return None

    roi_sum = float(roi.sum())
    if roi_sum <= 1e-6:
        return None

    roi_max = float(roi.max())
    roi_mean = float(roi.mean())
    roi_std = float(roi.std())
    threshold = max(roi_mean + roi_std * 0.75, roi_max * 0.6)
    salient_mask = roi >= threshold
    if not np.any(salient_mask):
        salient_mask = roi >= max(roi_mean + roi_std * 0.25, roi_max * 0.4)
    if not np.any(salient_mask):
        return None

    ys, xs = np.where(salient_mask)
    weights = roi[salient_mask].astype(np.float64)
    weight_sum = float(weights.sum())
    if weight_sum <= 1e-6:
        return None

    center_x = x1 + float(np.dot(xs, weights) / weight_sum)
    center_y = y1 + float(np.dot(ys, weights) / weight_sum)
    left = x1 + float(xs.min())
    top = y1 + float(ys.min())
    right = x1 + float(xs.max() + 1)
    bottom = y1 + float(ys.max() + 1)
    confidence = clamp(weight_sum / max(1.0, roi.size), 0.0, 1.0)
    return center_x, center_y, left, top, right, bottom, confidence


def load_speaker_segments(path: str) -> list[dict]:
    if not path:
        return []
    data = json.loads(Path(path).read_text(encoding="utf-8"))
    if isinstance(data, dict) and "segments" in data:
        return data["segments"]
    if isinstance(data, list):
        return data
    return []


def active_speaker_index(
    frame_idx: int,
    fps: float,
    segments: list[dict],
) -> Optional[int]:
    if not segments:
        return None
    t = frame_idx / max(fps, 1e-6)
    for seg in segments:
        if seg.get("start", -1) <= t <= seg.get("end", -1):
            try:
                return int(seg.get("speaker"))
            except (TypeError, ValueError):
                return None
    return None


def build_candidates(
    result: Any,
    frame: np.ndarray,
    saliency_map: np.ndarray,
    ranking_model: SubjectRankingModel,
    allowed_class_ids: list[int],
    class_names: dict[int, str],
    face_helper: MediaPipeFaceHelper,
    pose_helper: Optional["MediaPipePoseHelper"],
    state: CameraState,
    fps: float,
    frame_idx: int,
    speaker_segments: list[dict],
) -> list[Candidate]:
    h, w = frame.shape[:2]
    frame_cx = w / 2
    frame_cy = h / 2
    frame_area = float(h * w)
    frame_diag = math.hypot(w, h)

    boxes = result.boxes
    masks = result.masks
    masks_data = (
        masks.data.cpu().numpy()
        if masks is not None and getattr(masks, "data", None) is not None
        else None
    )

    candidates: list[Candidate] = []

    for i, box in enumerate(boxes):
        try:
            cls_id = int(box.cls[0].item())
            conf = float(box.conf[0].item())
            if cls_id not in allowed_class_ids:
                continue

            x1, y1, x2, y2 = map(float, box.xyxy[0].tolist())
            width = max(1.0, x2 - x1)
            height = max(1.0, y2 - y1)
            area = width * height
            cx = (x1 + x2) / 2
            cy = (y1 + y2) / 2
            track_id = get_track_id(box)
            cls_name = class_names.get(cls_id, str(cls_id))

            mask_area, mask_cx, mask_cy, mask_top_y = area, cx, cy, y1
            if masks_data is not None and i < len(masks_data):
                raw_mask = masks_data[i]
                if raw_mask.shape[:2] != frame.shape[:2]:
                    raw_mask = cv2.resize(
                        raw_mask, (w, h), interpolation=cv2.INTER_NEAREST
                    )
                binary_mask = (raw_mask > 0.5).astype(np.uint8)
                mask_area, mask_cx, mask_cy, mask_top_y = (
                    mask_stats_from_binary_mask(binary_mask)
                )
                if mask_area <= 0:
                    mask_area, mask_cx, mask_cy, mask_top_y = area, cx, cy, y1

            framing_cx = mask_cx
            framing_cy = mask_cy
            salient_box = extract_saliency_region(
                saliency_map, (int(x1), int(y1), int(x2), int(y2))
            )
            face_box = None
            pose_data: Optional[dict] = None

            if cls_name == "person":
                if pose_helper is not None:
                    pose_data = pose_helper.detect_in_person_box(
                        frame, (int(x1), int(y1), int(x2), int(y2))
                    )
                face_box = face_helper.detect_in_person_box(
                    frame, (int(x1), int(y1), int(x2), int(y2))
                )
                if face_box is not None:
                    fx1, fy1, fx2, fy2 = face_box
                    framing_cx = (fx1 + fx2) / 2
                    framing_cy = fy1 + (fy2 - fy1) * 0.42
                elif pose_data is not None and pose_data.get("body_cx") is not None:
                    framing_cx = float(pose_data["body_cx"])
                    eye_val = pose_data.get("eye_y")
                    shoulder_val = pose_data.get("shoulder_y")
                    if eye_val is not None and shoulder_val is not None:
                        framing_cy = (float(eye_val) + float(shoulder_val)) / 2
                    elif eye_val is not None:
                        framing_cy = float(eye_val)
                    elif shoulder_val is not None:
                        framing_cy = float(shoulder_val)
                    else:
                        framing_cy = mask_cy
                elif salient_box is not None:
                    framing_cx = salient_box[0]
                    framing_cy = salient_box[1]
                else:
                    framing_cx = mask_cx
                    framing_cy = mask_top_y + height * 0.28
            elif salient_box is not None:
                framing_cx = salient_box[0]
                framing_cy = salient_box[1]

            dist_center = math.hypot(
                framing_cx - frame_cx,
                framing_cy - frame_cy,
            )

            active_speaker = active_speaker_index(frame_idx, fps, speaker_segments)
            speaker_is_active = False
            if cls_name == "person":
                assumed_speaker = 0 if framing_cx < w / 2 else 1
                speaker_is_active = (
                    active_speaker is not None
                    and int(assumed_speaker) == int(active_speaker)
                )
            tracking_match = (
                state.tracked_id is not None
                and track_id == state.tracked_id
                and cls_id == state.tracked_cls_id
            )
            lock_match = (
                state.lock_track_id is not None
                and track_id == state.lock_track_id
                and cls_id == state.lock_cls_id
            )
            score = ranking_model.predict(
                cls_name=cls_name,
                conf=conf,
                mask_area=mask_area,
                frame_area=frame_area,
                dist_center=dist_center,
                frame_diag=frame_diag,
                has_face=face_box is not None,
                has_pose=pose_data is not None,
                saliency_confidence=(
                    salient_box[6] if salient_box is not None else 0.0
                ),
                tracking_match=tracking_match,
                lock_match=lock_match,
                speaker_active=speaker_is_active,
            )

            eye_y_val = None
            chin_y_val = None
            body_top_val = None
            body_bottom_val = None
            body_cx_val = None
            shoulder_span_val = None
            body_bottom_confident_val = False
            body_min_x_val = None
            body_max_x_val = None
            if pose_data is not None:
                eye_y_val = pose_data.get("eye_y")
                chin_y_val = pose_data.get("chin_y")
                body_top_val = pose_data.get("body_top_y")
                body_bottom_val = pose_data.get("body_bottom_y")
                body_cx_val = pose_data.get("body_cx")
                shoulder_span_val = pose_data.get("shoulder_span")
                body_bottom_confident_val = bool(
                    pose_data.get("body_bottom_confident", False)
                )
                body_min_x_val = pose_data.get("body_min_x")
                body_max_x_val = pose_data.get("body_max_x")

            candidates.append(
                Candidate(
                    cls_id=cls_id,
                    cls_name=cls_name,
                    track_id=track_id,
                    conf=conf,
                    x1=x1,
                    y1=y1,
                    x2=x2,
                    y2=y2,
                    cx=cx,
                    cy=cy,
                    width=width,
                    height=height,
                    area=area,
                    mask_area=mask_area,
                    mask_cx=mask_cx,
                    mask_cy=mask_cy,
                    mask_top_y=mask_top_y,
                    framing_cx=framing_cx,
                    framing_cy=framing_cy,
                    face_box=face_box,
                    score=score,
                    eye_y=eye_y_val,
                    chin_y=chin_y_val,
                    body_top_y=body_top_val,
                    body_bottom_y=body_bottom_val,
                    body_cx=body_cx_val,
                    shoulder_span=shoulder_span_val,
                    body_bottom_confident=body_bottom_confident_val,
                    body_min_x=body_min_x_val,
                    body_max_x=body_max_x_val,
                    salient_x1=(salient_box[2] if salient_box is not None else None),
                    salient_y1=(salient_box[3] if salient_box is not None else None),
                    salient_x2=(salient_box[4] if salient_box is not None else None),
                    salient_y2=(salient_box[5] if salient_box is not None else None),
                    saliency_confidence=(
                        salient_box[6] if salient_box is not None else 0.0
                    ),
                )
            )
        except Exception:
            continue

    candidates.sort(key=lambda c: c.score, reverse=True)
    return candidates


def choose_subject(
    candidates: list[Candidate],
    state: CameraState,
    lock_first_subject: bool,
    min_subject_hold_frames: int,
    switch_score_threshold: float,
) -> Optional[Candidate]:
    if not candidates:
        return None

    if lock_first_subject and state.lock_track_id is not None:
        for candidate in candidates:
            if (
                candidate.track_id == state.lock_track_id
                and candidate.cls_id == state.lock_cls_id
            ):
                return candidate

    best = candidates[0]
    previous = None
    for candidate in candidates:
        if (
            candidate.track_id == state.tracked_id
            and candidate.cls_id == state.tracked_cls_id
        ):
            previous = candidate
            break

    if previous is not None:
        if (
            previous.track_id == best.track_id
            and previous.cls_id == best.cls_id
        ):
            return previous

        if state.frames_since_subject_switch < min_subject_hold_frames:
            if previous.score * switch_score_threshold >= best.score:
                return previous

        if previous.score >= best.score * 0.92:
            return previous

    return best

def advance_value_with_velocity(
    current: float,
    target: float,
    velocity: float,
    response: float,
    damping: float,
    max_velocity: float,
) -> tuple[float, float]:
    velocity = (velocity * damping) + ((target - current) * response)
    velocity = clamp(velocity, -max_velocity, max_velocity)
    return current + velocity, velocity


def apply_camera_motion(
    state: CameraState,
    observation: CameraObservation,
    args: argparse.Namespace,
    base_crop_w: int,
    base_crop_h: int,
    frame_w: int,
    frame_h: int,
    x_min: Optional[float] = None,
    x_max: Optional[float] = None,
) -> tuple[int, int]:
    obs_weight = clamp(observation.confidence, 0.15, 1.0)
    obs_target_alpha = clamp(args.target_alpha * (0.55 + obs_weight * 0.65), 0.04, 0.4)

    state.target_zoom = lerp(state.target_zoom, observation.zoom, obs_target_alpha)
    state.zoom, state.zoom_velocity = advance_value_with_velocity(
        state.zoom,
        state.target_zoom,
        state.zoom_velocity,
        args.zoom_response,
        args.zoom_damping,
        args.zoom_alpha,
    )

    crop_w, crop_h = current_crop_size(
        base_crop_w,
        base_crop_h,
        state.zoom,
        frame_w,
        frame_h,
    )

    lo_x = crop_w / 2
    hi_x = frame_w - crop_w / 2
    if x_min is not None:
        lo_x = max(lo_x, x_min)
    if x_max is not None:
        hi_x = min(hi_x, x_max)
    if lo_x > hi_x:
        lo_x = hi_x = (lo_x + hi_x) / 2

    state.target_center_x = lerp(
        state.target_center_x,
        observation.center_x,
        obs_target_alpha,
    )
    state.target_center_y = lerp(
        state.target_center_y,
        observation.center_y,
        obs_target_alpha,
    )

    state.target_center_x = clamp(
        state.target_center_x,
        lo_x,
        hi_x,
    )
    state.target_center_y = clamp(
        state.target_center_y,
        crop_h / 2,
        frame_h - crop_h / 2,
    )

    next_center_x, state.velocity_x = advance_value_with_velocity(
        state.crop_center_x,
        state.target_center_x,
        state.velocity_x,
        args.motion_response * (0.65 + obs_weight * 0.35),
        clamp(args.motion_damping + 0.08, 0.0, 0.96),
        max(1.0, args.max_step_x * (0.35 + obs_weight * 0.65)),
    )
    next_center_y, state.velocity_y = advance_value_with_velocity(
        state.crop_center_y,
        state.target_center_y,
        state.velocity_y,
        args.motion_response * (0.7 + obs_weight * 0.3),
        clamp(args.motion_damping + 0.06, 0.0, 0.96),
        max(1.0, args.max_step_y * (0.35 + obs_weight * 0.65)),
    )

    state.crop_center_x = clamp(
        next_center_x,
        lo_x,
        hi_x,
    )
    state.crop_center_y = clamp(
        next_center_y,
        crop_h / 2,
        frame_h - crop_h / 2,
    )
    return crop_w, crop_h


def derive_candidate_focus_bounds(
    subject: Candidate,
) -> tuple[float, float, float, float]:
    if (
        subject.cls_name == "person"
        and subject.body_min_x is not None
        and subject.body_max_x is not None
        and subject.body_top_y is not None
        and subject.body_bottom_y is not None
    ):
        left = float(subject.body_min_x)
        right = float(subject.body_max_x)
        top = float(subject.body_top_y)
        bottom = float(subject.body_bottom_y)
        height = max(1.0, bottom - top)

        if subject.eye_y is not None:
            headroom = max(0.0, float(subject.eye_y) - top)
            top -= max(height * 0.10, headroom * 1.35)
        else:
            top -= height * 0.14

        if subject.body_bottom_confident:
            bottom += height * 0.08
        else:
            bottom += height * 0.18
    elif subject.cls_name == "person" and subject.face_box is not None:
        fx1, fy1, fx2, fy2 = subject.face_box
        face_w = max(1.0, fx2 - fx1)
        face_h = max(1.0, fy2 - fy1)
        left = fx1 - face_w * 1.15
        right = fx2 + face_w * 1.15
        top = fy1 - face_h * 1.55
        bottom = fy2 + face_h * 4.0
    else:
        left = subject.x1
        right = subject.x2
        top = subject.mask_top_y
        bottom = subject.y2
        height = max(1.0, bottom - top)
        width = max(1.0, right - left)
        left -= width * 0.14
        right += width * 0.14
        top -= height * 0.14
        bottom += height * 0.16

    if (
        subject.salient_x1 is not None
        and subject.salient_y1 is not None
        and subject.salient_x2 is not None
        and subject.salient_y2 is not None
    ):
        saliency_pad_x = max(
            8.0, (subject.salient_x2 - subject.salient_x1) * 0.18
        )
        saliency_pad_y = max(
            8.0, (subject.salient_y2 - subject.salient_y1) * 0.22
        )
        left = min(left, subject.salient_x1 - saliency_pad_x)
        top = min(top, subject.salient_y1 - saliency_pad_y)
        right = max(right, subject.salient_x2 + saliency_pad_x)
        bottom = max(bottom, subject.salient_y2 + saliency_pad_y)

    return left, top, right, bottom


def compute_observation_from_bounds(
    bounds: tuple[float, float, float, float],
    confidence: float,
    base_crop_w: int,
    base_crop_h: int,
    frame_w: int,
    frame_h: int,
    min_zoom: float,
    max_zoom: float,
) -> CameraObservation:
    left, top, right, bottom = bounds
    left = clamp(left, 0.0, frame_w - 1.0)
    top = clamp(top, 0.0, frame_h - 1.0)
    right = clamp(right, left + 1.0, float(frame_w))
    bottom = clamp(bottom, top + 1.0, float(frame_h))

    region_w = max(1.0, right - left)
    region_h = max(1.0, bottom - top)
    pad_x = max(region_w * 0.12, 16.0)
    pad_y = max(region_h * 0.16, 20.0)

    fit_w = min(frame_w, region_w + pad_x * 2.0)
    fit_h = min(frame_h, region_h + pad_y * 2.0)
    zoom_w = base_crop_w / max(1.0, fit_w)
    zoom_h = base_crop_h / max(1.0, fit_h)
    zoom = clamp(min(zoom_w, zoom_h), min_zoom, max_zoom)

    crop_w, crop_h = current_crop_size(
        base_crop_w,
        base_crop_h,
        zoom,
        frame_w,
        frame_h,
    )
    center_x = clamp((left + right) / 2.0, crop_w / 2, frame_w - crop_w / 2)
    center_y = clamp((top + bottom) / 2.0, crop_h / 2, frame_h - crop_h / 2)

    return CameraObservation(
        center_x=center_x,
        center_y=center_y,
        zoom=zoom,
        confidence=confidence,
    )


def build_single_subject_observation(
    subject: Candidate,
    base_crop_w: int,
    base_crop_h: int,
    frame_w: int,
    frame_h: int,
    min_zoom: float,
    max_zoom: float,
) -> CameraObservation:
    bounds = derive_candidate_focus_bounds(subject)
    confidence = clamp(
        subject.conf * 0.60
        + min(1.0, subject.score / 400.0) * 0.20
        + subject.saliency_confidence * 0.20,
        0.2,
        1.0,
    )
    return compute_observation_from_bounds(
        bounds=bounds,
        confidence=confidence,
        base_crop_w=base_crop_w,
        base_crop_h=base_crop_h,
        frame_w=frame_w,
        frame_h=frame_h,
        min_zoom=min_zoom,
        max_zoom=max_zoom,
    )


def build_pair_observation(
    first: Candidate,
    second: Candidate,
    base_crop_w: int,
    base_crop_h: int,
    frame_w: int,
    frame_h: int,
    min_zoom: float,
    max_zoom: float,
) -> CameraObservation:
    first_bounds = derive_candidate_focus_bounds(first)
    second_bounds = derive_candidate_focus_bounds(second)
    union_bounds = (
        min(first_bounds[0], second_bounds[0]),
        min(first_bounds[1], second_bounds[1]),
        max(first_bounds[2], second_bounds[2]),
        max(first_bounds[3], second_bounds[3]),
    )
    confidence = clamp((first.conf + second.conf) / 2.0, 0.25, 1.0)
    return compute_observation_from_bounds(
        bounds=union_bounds,
        confidence=confidence,
        base_crop_w=base_crop_w,
        base_crop_h=base_crop_h,
        frame_w=frame_w,
        frame_h=frame_h,
        min_zoom=min_zoom,
        max_zoom=max_zoom,
    )


def build_global_saliency_observation(
    saliency_map: np.ndarray,
    base_crop_w: int,
    base_crop_h: int,
    frame_w: int,
    frame_h: int,
    min_zoom: float,
    max_zoom: float,
) -> Optional[CameraObservation]:
    region = extract_saliency_region(saliency_map, (0, 0, frame_w, frame_h))
    if region is None or region[6] < 0.02:
        return None
    return compute_observation_from_bounds(
        bounds=(region[2], region[3], region[4], region[5]),
        confidence=clamp(region[6], 0.12, 0.55),
        base_crop_w=base_crop_w,
        base_crop_h=base_crop_h,
        frame_w=frame_w,
        frame_h=frame_h,
        min_zoom=min_zoom,
        max_zoom=max_zoom,
    )


def choose_two_person_pair(
    candidates: list[Candidate], threshold: float
) -> Optional[tuple[Candidate, Candidate]]:
    persons = [
        candidate for candidate in candidates if candidate.cls_name == "person"
    ]
    if len(persons) < 2:
        return None

    first, second = persons[0], persons[1]
    if second.score < first.score * threshold:
        return None
    return first, second


def crop_frame(
    frame: np.ndarray,
    center_x: float,
    center_y: float,
    crop_w: int,
    crop_h: int,
) -> tuple[np.ndarray, tuple[int, int, int, int]]:
    frame_h, frame_w = frame.shape[:2]
    left = int(round(center_x - crop_w / 2))
    top = int(round(center_y - crop_h / 2))

    left = int(clamp(left, 0, frame_w - crop_w))
    top = int(clamp(top, 0, frame_h - crop_h))
    right = left + crop_w
    bottom = top + crop_h
    return frame[top:bottom, left:right], (left, top, right, bottom)


def resize_cover(
    img: np.ndarray,
    target_w: int,
    target_h: int,
) -> np.ndarray:
    """Resize an image to fill the target while preserving aspect ratio
    (center-crop the overflow, like CSS object-fit: cover)."""
    ih, iw = img.shape[:2]
    if iw <= 0 or ih <= 0:
        return img
    scale = max(target_w / iw, target_h / ih)
    nw = max(1, int(round(iw * scale)))
    nh = max(1, int(round(ih * scale)))
    resized = cv2.resize(img, (nw, nh), interpolation=cv2.INTER_LINEAR)
    x = max(0, (nw - target_w) // 2)
    y = max(0, (nh - target_h) // 2)
    return resized[y : y + target_h, x : x + target_w]


def assign_panes(
    candidates: list[Candidate],
    left_key: Optional[tuple[Optional[int], int]],
    right_key: Optional[tuple[Optional[int], int]],
) -> Optional[tuple[Candidate, Candidate]]:
    """Pick the two people for split-screen panes, preferring previously
    assigned track IDs and falling back to left/right position."""
    persons = [c for c in candidates if c.cls_name == "person"]
    if len(persons) < 2:
        return None

    left = None
    right = None

    for c in persons:
        if (
            left_key is not None
            and left_key[0] is not None
            and c.track_id == left_key[0]
            and c.cls_id == left_key[1]
        ):
            left = c
        if (
            right_key is not None
            and right_key[0] is not None
            and c.track_id == right_key[0]
            and c.cls_id == right_key[1]
        ):
            right = c

    remaining = [c for c in persons if c is not left and c is not right]
    if left is None and remaining:
        remaining.sort(key=lambda c: c.framing_cx)
        left = remaining.pop(0)
    if right is None and remaining:
        remaining = [c for c in persons if c is not left and c is not right]
        if remaining:
            remaining.sort(key=lambda c: -c.framing_cx)
            right = remaining.pop(0)

    if left is None or right is None:
        return None
    if left.track_id == right.track_id and left.track_id is not None:
        return None
    if left is right:
        return None
    return left, right


def draw_debug(
    frame: np.ndarray,
    crop_rect: tuple[int, int, int, int],
    candidates: list[Candidate],
    subject: Optional[Candidate],
    pair: Optional[tuple[Candidate, Candidate]],
    state: CameraState,
    frame_idx: int,
    total_frames: int,
) -> np.ndarray:
    dbg = frame.copy()
    left, top, right, bottom = crop_rect
    cv2.rectangle(dbg, (left, top), (right, bottom), (0, 255, 255), 2)

    for candidate in candidates[:6]:
        color = (120, 120, 120)
        thickness = 1
        if (
            subject is not None
            and candidate.track_id == subject.track_id
            and candidate.cls_id == subject.cls_id
        ):
            color = (0, 255, 0)
            thickness = 2
        cv2.rectangle(
            dbg,
            (int(candidate.x1), int(candidate.y1)),
            (int(candidate.x2), int(candidate.y2)),
            color,
            thickness,
        )
        if candidate.face_box is not None:
            fx1, fy1, fx2, fy2 = candidate.face_box
            cv2.rectangle(dbg, (fx1, fy1), (fx2, fy2), (255, 0, 0), 2)

    if pair is not None:
        for candidate in pair:
            cv2.circle(
                dbg,
                (int(candidate.framing_cx), int(candidate.framing_cy)),
                8,
                (255, 0, 255),
                -1,
            )

    lines = [
        f"frame {frame_idx}/{total_frames if total_frames > 0 else '?'}",
        f"scene={state.current_scene_index}",
        f"tracked_id={state.tracked_id} cls={state.tracked_cls_id}",
        f"lock_id={state.lock_track_id} zoom={state.zoom:.2f}",
    ]
    y = 28
    for line in lines:
        cv2.putText(
            dbg,
            line,
            (16, y),
            cv2.FONT_HERSHEY_SIMPLEX,
            0.7,
            (255, 255, 255),
            2,
            cv2.LINE_AA,
        )
        y += 28
    return dbg


def has_ffmpeg_encoder(encoder_name: str) -> bool:
    if shutil.which("ffmpeg") is None:
        return False
    result = subprocess.run(
        ["ffmpeg", "-hide_banner", "-encoders"],
        capture_output=True,
        text=True,
        check=False,
    )
    return encoder_name in result.stdout


def build_video_filters(
    post_restore: bool,
) -> Optional[str]:
    filters = []

    if post_restore:
        filters.append("hqdn3d=1.2:1.2:6:6")
        filters.append("unsharp=5:5:0.6:5:5:0.0")

    if not filters:
        return None
    return ",".join(filters)


def run_ffmpeg_mux(
    silent_video_path: str,
    source_input_path: str,
    final_output_path: str,
    video_encoder: str,
    audio_bitrate: str,
    crf: int,
    preset: str,
    vf: Optional[str],
) -> None:
    if shutil.which("ffmpeg") is None:
        raise RuntimeError("ffmpeg not found in PATH")

    encoder = video_encoder
    if encoder and not has_ffmpeg_encoder(encoder):
        logging.warning("Encoder %s not found, fallback to libx264", encoder)
        encoder = "libx264"

    final_path = Path(final_output_path)
    mux_output_path = Path(silent_video_path).with_name(
        f"{final_path.stem}_mux{final_path.suffix or '.mp4'}"
    )
    mux_output_path.unlink(missing_ok=True)

    cmd = [
        "ffmpeg",
        "-y",
        "-i",
        silent_video_path,
        "-i",
        source_input_path,
        "-map",
        "0:v:0",
        "-map",
        "1:a?",
    ]

    if vf:
        cmd += ["-vf", vf]

    cmd += ["-c:v", encoder]

    if encoder == "libx264":
        cmd += ["-crf", str(crf), "-preset", preset, "-pix_fmt", "yuv420p"]
    elif encoder in {"h264_videotoolbox", "hevc_videotoolbox"}:
        cmd += ["-pix_fmt", "yuv420p"]

    cmd += [
        "-c:a",
        "aac",
        "-b:a",
        audio_bitrate,
        "-shortest",
        str(mux_output_path),
    ]

    result = subprocess.run(cmd, capture_output=True, text=True, check=False)
    if result.returncode != 0:
        raise RuntimeError(result.stderr)

    try:
        shutil.move(str(mux_output_path), final_output_path)
    except OSError as exc:
        raise RuntimeError(
            f"Could not move rendered video to {final_output_path}: {exc}"
        ) from exc


def reset_for_new_scene(
    state: CameraState,
    frame_w: int,
    frame_h: int,
    min_zoom: float,
) -> None:
    state.crop_center_x = frame_w / 2
    state.crop_center_y = frame_h / 2
    state.zoom = min_zoom
    state.target_center_x = frame_w / 2
    state.target_center_y = frame_h / 2
    state.target_zoom = min_zoom
    state.velocity_x = 0.0
    state.velocity_y = 0.0
    state.zoom_velocity = 0.0
    state.tracked_id = None
    state.tracked_cls_id = None
    state.missed_frames = 0
    state.lock_track_id = None
    state.lock_cls_id = None
    state.frames_since_subject_switch = 0
    state.last_framing_cx = None
    state.last_framing_cy = None
    state.last_subject_key = None
    state.framing_vx = 0.0
    state.framing_vy = 0.0


def process_video(args: argparse.Namespace) -> None:
    input_path = Path(args.input)
    output_path = Path(args.output)

    cap = cv2.VideoCapture(str(input_path))
    if not cap.isOpened():
        raise RuntimeError(f"Could not open input video: {input_path}")

    fps = cap.get(cv2.CAP_PROP_FPS) or 30.0
    frame_w = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    frame_h = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))
    total_frames = int(cap.get(cv2.CAP_PROP_FRAME_COUNT))
    cap.release()

    base_crop_w, base_crop_h = compute_base_crop(frame_w, frame_h, 9, 16)
    allowed_class_ids = [
        CLASS_IDS[name] for name in args.classes if name in CLASS_IDS
    ]
    if not allowed_class_ids:
        raise ValueError("No valid classes selected")

    speaker_segments = load_speaker_segments(args.speaker_json)
    scene_start_frames = detect_scenes(
        str(input_path), args.scene_threshold, args.min_scene_len
    )
    scene_start_set = set(scene_start_frames)

    model = YOLO(args.seg_model)
    class_names = model.names
    face_helper = MediaPipeFaceHelper()
    pose_helper = MediaPipePoseHelper()
    saliency_helper = build_saliency_helper(args)
    ranking_model = SubjectRankingModel()
    if pose_helper.detector is None:
        pose_helper = None

    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir_path = Path(tmpdir)
        silent_video_path = tmpdir_path / "silent_vertical.mp4"
        debug_video_path = tmpdir_path / "debug_vertical.mp4"

        fourcc = cv2.VideoWriter_fourcc(*"mp4v")
        writer = cv2.VideoWriter(
            str(silent_video_path),
            fourcc,
            fps,
            (args.output_width, args.output_height),
        )
        if not writer.isOpened():
            raise RuntimeError("Could not create temporary output video")

        debug_writer = None
        final_debug_path = None
        if args.save_debug_preview:
            final_debug_path = (
                Path(args.debug_path)
                if args.debug_path
                else output_path.with_name(output_path.stem + "_debug.mp4")
            )
            debug_writer = cv2.VideoWriter(
                str(debug_video_path),
                fourcc,
                fps,
                (args.output_width, args.output_height),
            )
            if not debug_writer.isOpened():
                raise RuntimeError("Could not create debug preview video")

        state = CameraState(
            crop_center_x=frame_w / 2,
            crop_center_y=frame_h / 2,
            zoom=args.min_zoom,
            target_center_x=frame_w / 2,
            target_center_y=frame_h / 2,
            target_zoom=args.min_zoom,
        )
        left_state = CameraState(
            crop_center_x=frame_w / 4,
            crop_center_y=frame_h / 2,
            zoom=args.min_zoom,
            target_center_x=frame_w / 4,
            target_center_y=frame_h / 2,
            target_zoom=args.min_zoom,
        )
        right_state = CameraState(
            crop_center_x=frame_w * 3 / 4,
            crop_center_y=frame_h / 2,
            zoom=args.min_zoom,
            target_center_x=frame_w * 3 / 4,
            target_center_y=frame_h / 2,
            target_zoom=args.min_zoom,
        )
        pane_left_key: Optional[tuple[Optional[int], int]] = None
        pane_right_key: Optional[tuple[Optional[int], int]] = None
        auto_split_frames = 0
        use_split = args.layout == "split"

        stats = {
            "frames_processed": 0,
            "scene_resets": 0,
            "subject_switches": 0,
            "frames_with_subject": 0,
            "frames_with_face": 0,
            "frames_with_two_person": 0,
            "frames_split": 0,
            "frames_single": 0,
        }

        last_subject_key = None
        scene_index = 0

        results = model.track(
            source=str(input_path),
            stream=True,
            persist=True,
            tracker=args.tracker,
            classes=allowed_class_ids,
            conf=args.conf,
            retina_masks=True,
            verbose=False,
        )

        for frame_idx, result in enumerate(results, start=1):
            frame = result.orig_img
            if frame is None:
                continue

            if frame_idx in scene_start_set:
                scene_index += 1
                state.current_scene_index = scene_index
                reset_for_new_scene(state, frame_w, frame_h, args.min_zoom)
                reset_for_new_scene(left_state, frame_w, frame_h, args.min_zoom)
                reset_for_new_scene(right_state, frame_w, frame_h, args.min_zoom)
                left_state.crop_center_x = frame_w / 4
                left_state.target_center_x = frame_w / 4
                right_state.crop_center_x = frame_w * 3 / 4
                right_state.target_center_x = frame_w * 3 / 4
                saliency_helper.reset_temporal_state()
                stats["scene_resets"] += 1
                last_subject_key = None
                pane_left_key = None
                pane_right_key = None
                auto_split_frames = 0

            saliency_map = saliency_helper.compute_map(frame)
            candidates = build_candidates(
                result=result,
                frame=frame,
                saliency_map=saliency_map,
                ranking_model=ranking_model,
                allowed_class_ids=allowed_class_ids,
                class_names=class_names,
                face_helper=face_helper,
                pose_helper=pose_helper,
                state=state,
                fps=fps,
                frame_idx=frame_idx,
                speaker_segments=(
                    speaker_segments if args.speaker_aware_mode else []
                ),
            )

            subject = choose_subject(
                candidates,
                state,
                args.lock_first_subject,
                args.min_subject_hold_frames,
                args.switch_score_threshold,
            )
            pair = None
            if args.layout in ("split", "auto"):
                pane_pair = assign_panes(candidates, pane_left_key, pane_right_key)
                if args.layout == "split":
                    use_split = pane_pair is not None
                else:
                    if pane_pair is not None:
                        auto_split_frames += 2
                    else:
                        auto_split_frames = max(0, auto_split_frames - 1)
                    use_split = auto_split_frames >= 14
                if use_split and pane_pair is not None:
                    pair = pane_pair

            if (
                not use_split
                and args.speaker_aware_mode
                and speaker_segments
                and subject is not None
            ):
                active_spk = active_speaker_index(frame_idx, fps, speaker_segments)
                if active_spk is not None:
                    subj_half = 0 if subject.framing_cx < frame_w / 2 else 1
                    if (
                        int(subj_half) != int(active_spk)
                        and state.frames_since_subject_switch
                        >= args.min_subject_hold_frames
                    ):
                        target = None
                        for cand in candidates:
                            if cand.cls_name != "person":
                                continue
                            cand_half = 0 if cand.framing_cx < frame_w / 2 else 1
                            if cand_half == int(active_spk):
                                if target is None or cand.score > target.score:
                                    target = cand
                        if target is not None:
                            subject = target

            rendered_split = False
            clean_out = None
            crop_rect = (0, 0, frame_w, frame_h)

            if pair is not None:
                stats["frames_with_two_person"] += 1
                stats["frames_split"] += 1
                left_cand, right_cand = pair
                if left_cand.framing_cx > right_cand.framing_cx:
                    left_cand, right_cand = right_cand, left_cand
                if left_cand.track_id is not None:
                    pane_left_key = (left_cand.track_id, left_cand.cls_id)
                if right_cand.track_id is not None:
                    pane_right_key = (right_cand.track_id, right_cand.cls_id)

                left_obs = build_single_subject_observation(
                    subject=left_cand,
                    base_crop_w=base_crop_w,
                    base_crop_h=base_crop_h,
                    frame_w=frame_w,
                    frame_h=frame_h,
                    min_zoom=args.min_zoom,
                    max_zoom=args.max_zoom,
                )
                apply_camera_motion(
                    left_state,
                    left_obs,
                    args,
                    base_crop_w,
                    base_crop_h,
                    frame_w,
                    frame_h,
                    x_min=0.0,
                    x_max=frame_w / 2,
                )
                crop_w, crop_h = current_crop_size(
                    base_crop_w, base_crop_h, left_state.zoom, frame_w, frame_h
                )
                left_crop, _ = crop_frame(
                    frame,
                    left_state.crop_center_x,
                    left_state.crop_center_y,
                    crop_w,
                    crop_h,
                )
                divider_w = 0
                if args.split_divider:
                    divider_w = max(1, args.output_width // 540)
                pane_w = max(1, (args.output_width - divider_w) // 2)
                left_pane = resize_cover(left_crop, pane_w, args.output_height)

                right_obs = build_single_subject_observation(
                    subject=right_cand,
                    base_crop_w=base_crop_w,
                    base_crop_h=base_crop_h,
                    frame_w=frame_w,
                    frame_h=frame_h,
                    min_zoom=args.min_zoom,
                    max_zoom=args.max_zoom,
                )
                apply_camera_motion(
                    right_state,
                    right_obs,
                    args,
                    base_crop_w,
                    base_crop_h,
                    frame_w,
                    frame_h,
                    x_min=frame_w / 2,
                    x_max=float(frame_w),
                )
                crop_w, crop_h = current_crop_size(
                    base_crop_w, base_crop_h, right_state.zoom, frame_w, frame_h
                )
                right_crop, _ = crop_frame(
                    frame,
                    right_state.crop_center_x,
                    right_state.crop_center_y,
                    crop_w,
                    crop_h,
                )
                right_pane = resize_cover(right_crop, pane_w, args.output_height)

                if divider_w > 0:
                    divider = np.zeros(
                        (args.output_height, divider_w, 3), dtype=np.uint8
                    )
                    clean_out = np.hstack([left_pane, divider, right_pane])
                else:
                    clean_out = np.hstack([left_pane, right_pane])

                if clean_out.shape[1] != args.output_width:
                    clean_out = cv2.resize(
                        clean_out,
                        (args.output_width, args.output_height),
                        interpolation=cv2.INTER_LINEAR,
                    )

                rendered_split = True
                state.missed_frames = 0
                state.tracked_id = None
                state.tracked_cls_id = None
                state.frames_since_subject_switch = 0
                state.last_framing_cx = None
                state.last_framing_cy = None
                state.last_subject_key = None
                state.framing_vx = 0.0
                state.framing_vy = 0.0

            elif subject is not None:
                stats["frames_with_subject"] += 1
                stats["frames_single"] += 1
                previous_subject_key = (state.tracked_id, state.tracked_cls_id)
                current_subject_key = (subject.track_id, subject.cls_id)
                if (
                    last_subject_key is not None
                    and current_subject_key != last_subject_key
                ):
                    stats["subject_switches"] += 1
                last_subject_key = current_subject_key

                if subject.face_box is not None:
                    stats["frames_with_face"] += 1

                if (
                    args.lock_first_subject
                    and state.lock_track_id is None
                    and subject.track_id is not None
                ):
                    state.lock_track_id = subject.track_id
                    state.lock_cls_id = subject.cls_id

                observation = build_single_subject_observation(
                    subject=subject,
                    base_crop_w=base_crop_w,
                    base_crop_h=base_crop_h,
                    frame_w=frame_w,
                    frame_h=frame_h,
                    min_zoom=args.min_zoom,
                    max_zoom=args.max_zoom,
                )

                subject_key = (subject.track_id, subject.cls_id)
                if (
                    state.last_subject_key == subject_key
                    and state.last_framing_cx is not None
                    and state.last_framing_cy is not None
                ):
                    raw_vx = subject.framing_cx - state.last_framing_cx
                    raw_vy = subject.framing_cy - state.last_framing_cy
                    state.framing_vx = state.framing_vx * 0.6 + raw_vx * 0.4
                    state.framing_vy = state.framing_vy * 0.6 + raw_vy * 0.4
                else:
                    state.framing_vx = 0.0
                    state.framing_vy = 0.0

                velocity_scale = clamp(observation.confidence, 0.0, 1.0)
                observation.center_x = clamp(
                    observation.center_x + state.framing_vx * velocity_scale * 0.8,
                    0.0,
                    float(frame_w),
                )
                observation.center_y = clamp(
                    observation.center_y
                    + state.framing_vy * velocity_scale * 0.45,
                    0.0,
                    float(frame_h),
                )

                state.last_framing_cx = subject.framing_cx
                state.last_framing_cy = subject.framing_cy
                state.last_subject_key = subject_key

                crop_w, crop_h = apply_camera_motion(
                    state,
                    observation,
                    args,
                    base_crop_w,
                    base_crop_h,
                    frame_w,
                    frame_h,
                )

                state.tracked_id = subject.track_id
                state.tracked_cls_id = subject.cls_id
                state.missed_frames = 0
                if current_subject_key == previous_subject_key:
                    state.frames_since_subject_switch += 1
                else:
                    state.frames_since_subject_switch = 0
            else:
                stats["frames_single"] += 1
                state.missed_frames += 1
                state.framing_vx *= 0.5
                state.framing_vy *= 0.5
                observation = build_global_saliency_observation(
                    saliency_map=saliency_map,
                    base_crop_w=base_crop_w,
                    base_crop_h=base_crop_h,
                    frame_w=frame_w,
                    frame_h=frame_h,
                    min_zoom=args.min_zoom,
                    max_zoom=args.max_zoom,
                )

                if observation is not None:
                    crop_w, crop_h = apply_camera_motion(
                        state,
                        observation,
                        args,
                        base_crop_w,
                        base_crop_h,
                        frame_w,
                        frame_h,
                    )
                else:
                    crop_w, crop_h = current_crop_size(
                        base_crop_w, base_crop_h, state.zoom, frame_w, frame_h
                    )
                    if state.missed_frames > args.max_missed_frames:
                        state.crop_center_x = lerp(
                            state.crop_center_x,
                            frame_w / 2,
                            0.03,
                        )
                        state.crop_center_y = lerp(
                            state.crop_center_y,
                            frame_h / 2,
                            0.03,
                        )
                        state.zoom = lerp(state.zoom, args.min_zoom, 0.05)
                        state.target_zoom = lerp(
                            state.target_zoom,
                            args.min_zoom,
                            0.05,
                        )
                    state.tracked_id = None
                    state.tracked_cls_id = None
                    state.frames_since_subject_switch = 0

                state.crop_center_x = clamp(
                    state.crop_center_x,
                    crop_w / 2,
                    frame_w - crop_w / 2,
                )
                state.crop_center_y = clamp(
                    state.crop_center_y,
                    crop_h / 2,
                    frame_h - crop_h / 2,
                )

            if rendered_split:
                writer.write(clean_out)
                if debug_writer is not None:
                    debug_writer.write(clean_out)
            else:
                crop_w, crop_h = current_crop_size(
                    base_crop_w, base_crop_h, state.zoom, frame_w, frame_h
                )
                cropped, crop_rect = crop_frame(
                    frame, state.crop_center_x, state.crop_center_y, crop_w, crop_h
                )
                clean_out = cv2.resize(
                    cropped,
                    (args.output_width, args.output_height),
                    interpolation=cv2.INTER_LINEAR,
                )
                writer.write(clean_out)

                if debug_writer is not None:
                    dbg = draw_debug(
                        frame,
                        crop_rect,
                        candidates,
                        subject,
                        pair if not use_split else None,
                        state,
                        frame_idx,
                        total_frames,
                    )
                    dbg_out = cv2.resize(
                        dbg,
                        (args.output_width, args.output_height),
                        interpolation=cv2.INTER_LINEAR,
                    )
                    debug_writer.write(dbg_out)

            stats["frames_processed"] += 1

            if frame_idx % 50 == 0:
                saliency_telemetry = saliency_helper.get_telemetry()
                logging.info(
                    "Processed %s/%s | scene=%s | layout=%s | zoom=%.2f | tracked=%s | saliency=%s/%s",
                    frame_idx,
                    total_frames if total_frames > 0 else "?",
                    state.current_scene_index,
                    "split" if rendered_split else "single",
                    state.zoom,
                    state.tracked_id,
                    saliency_telemetry.get("active_backend"),
                    saliency_telemetry.get("requested_backend"),
                )

        writer.release()
        if debug_writer is not None:
            debug_writer.release()

        vf = build_video_filters(
            post_restore=args.post_restore,
        )

        run_ffmpeg_mux(
            silent_video_path=str(silent_video_path),
            source_input_path=str(input_path),
            final_output_path=str(output_path),
            video_encoder=args.video_encoder,
            audio_bitrate=args.audio_bitrate,
            crf=args.crf,
            preset=args.preset_ffmpeg,
            vf=vf,
        )

        if debug_writer is not None and final_debug_path is not None:
            run_ffmpeg_mux(
                silent_video_path=str(debug_video_path),
                source_input_path=str(input_path),
                final_output_path=str(final_debug_path),
                video_encoder=args.video_encoder,
                audio_bitrate=args.audio_bitrate,
                crf=args.crf,
                preset=args.preset_ffmpeg,
                vf=None,
            )

        saliency_telemetry = saliency_helper.get_telemetry()
        summary = {
            "preset": args.preset,
            "frames_processed": stats["frames_processed"],
            "scene_resets": stats["scene_resets"],
            "frames_with_subject": stats["frames_with_subject"],
            "frames_with_face": stats["frames_with_face"],
            "frames_with_two_person": stats["frames_with_two_person"],
            "frames_split": stats["frames_split"],
            "frames_single": stats["frames_single"],
            "layout": args.layout,
            "subject_switches": stats["subject_switches"],
            "output_width": args.output_width,
            "output_height": args.output_height,
            "post_restore": args.post_restore,
            "saliency_requested_backend": saliency_telemetry.get(
                "requested_backend"
            ),
            "saliency_active_backend": saliency_telemetry.get(
                "active_backend"
            ),
            "saliency_model_loaded": saliency_telemetry.get("model_loaded"),
            "saliency_frames_total": saliency_telemetry.get("frames_total"),
            "saliency_frames_backend": saliency_telemetry.get("frames_backend"),
            "saliency_frames_fallback": saliency_telemetry.get(
                "frames_fallback"
            ),
            "saliency_device": saliency_telemetry.get("device"),
        }
        logging.info("Summary: %s", json.dumps(summary, ensure_ascii=False))


def main() -> int:
    args = parse_args()
    args = apply_preset(args)
    setup_logging(args.log_level)

    try:
        process_video(args)
        return 0
    except KeyboardInterrupt:
        logging.error("Interrupted by user")
        return 130
    except Exception as exc:
        logging.exception("Failed: %s", exc)
        return 1
