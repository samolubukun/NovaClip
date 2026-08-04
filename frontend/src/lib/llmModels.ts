export const GEMINI_MODELS = [
  { id: "gemini-3.1-flash-lite", label: "Gemini 3.1 Flash-Lite (Default)" },
  { id: "gemini-3.1-pro", label: "Gemini 3.1 Pro" },
];

export const OPENROUTER_MODELS = [
  { id: "openrouter/free", label: "Free Router (Text)" },
  { id: "nvidia/nemotron-3-nano-30b-a3b:free", label: "Nemotron 3 Nano (Free)" },
  { id: "poolside/laguna-s-2.1:free", label: "Poolside Laguna (Free)" },
  { id: "google/gemma-4-26b-a4b-it:free", label: "Gemma 4 26B Vision (Free)" },
  { id: "nvidia/nemotron-nano-12b-v2-vl:free", label: "Nemotron Nano VL (Free)" },
  { id: "nvidia/nemotron-3-nano-omni-30b-a3b-reasoning:free", label: "Nemotron Omni Vision (Free)" },
  { id: "custom", label: "Custom Model" },
];

export type LlmProvider = "gemini" | "openrouter";

export const OPENROUTER_VISION_MODEL_OPTIONS = [
  { id: "google/gemma-4-26b-a4b-it:free", label: "Gemma 4 26B Vision (Free)" },
  { id: "nvidia/nemotron-nano-12b-v2-vl:free", label: "Nemotron Nano VL (Free)" },
  { id: "nvidia/nemotron-3-nano-omni-30b-a3b-reasoning:free", label: "Nemotron Omni Vision (Free)" },
];

export const OPENROUTER_VISION_MODELS = new Set(OPENROUTER_VISION_MODEL_OPTIONS.map(model => model.id));
