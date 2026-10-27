# Small self-hosted tutors — research checked 2026-10-26

Start an application-specific comparison with **OmniEdu-9B**, **OmniEdu-4B** for tighter memory, and **Qwen3.5-9B** as a general-purpose baseline. This is a shortlist, not a universal ranking. No provider was changed or downloaded automatically.

[OmniEdu project and reported evaluations](https://haolpku.github.io/Omni-Edu/) publishes education-focused 4B/9B/27B checkpoints. The reported 9B TutorBench score is 48.16 versus 46.67 for 4B; other tutoring metrics do not improve uniformly with size. Original BF16 weight downloads are 8.46 GiB (4B) and 17.53 GiB (9B); released FP8 downloads are 5.14 and 11.09 GiB. These are weight sizes, not total runtime memory. Results are author-reported, with no proof of improved classroom outcomes.

The [9B model card](https://huggingface.co/lhpku20010120/Omni-Edu-9B) documents Transformers/vLLM serving. Use an OpenAI-compatible local or remote server profile in StudyHub. Native app tool JSON compliance, supported quantization formats, and latency still need measurement against the actual server. Do not assume an Ollama registry tag exists for this checkpoint. See the model's license and inherited dataset terms before deployment.

[Qwen3.5-9B's official card](https://huggingface.co/Qwen/Qwen3.5-9B) provides Apache-2.0 weights, local serving instructions, and quantization links. It is the practical general-purpose baseline; this research does not establish that it beats OmniEdu in this application.

Select a model using the same notes and tasks: one question per turn, unsupported-method refusal, factual grading, new numerical examples using taught methods, valid artifact JSON, and response latency. Include the reported x²+3x+5 case: no discriminant or quadratic formula if the selected notes do not explain those methods. GPU memory, context length and concurrency determine a suitable precision and size.
