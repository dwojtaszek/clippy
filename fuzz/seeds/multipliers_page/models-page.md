# Available Models

Models offered natively in the Factory platform.

The **Reasoning** column lists the values each model accepts for `reasoningEffort` in [settings](/droid-cli/settings#reasoning-effort) and for `--reasoning-effort` on the command line. The model selector labels them differently, so `xhigh` appears there as **Extra High**.

## <span className="provider-heading"><img src="/docs-assets/images/model-icons/anthropic.png" alt="Anthropic logo" className="provider-icon anthropic-icon" />Anthropic</span>

| Model | Model ID | Multiplier | Reasoning |
| --- | --- | --- | --- |
| Claude Fable 5.1<sup>\*</sup> | `claude-fable-5.1` | 4× | `off`, `low`, `medium`, `high` (default), `xhigh`, `max` |
| Claude Fable 5<sup>\*</sup> | `claude-fable-5` | 4× | `off`, `low`, `medium`, `high` (default), `xhigh`, `max` |
| Claude Opus 5.5 | `claude-opus-5-5` | 1.6× | `low`, `medium` (default), `high`, `xhigh`, `max` |
| Claude Opus 5.5 Fast | `claude-opus-5-5-fast` | 3.2× | `low`, `medium` (default), `high`, `xhigh`, `max` |
| Claude Opus 5 | `claude-opus-5` | 2× | `off`, `low`, `medium`, `high` (default), `xhigh`, `max` |
| Claude Opus 5 Fast | `claude-opus-5-fast` | 4× | `off`, `low`, `medium`, `high` (default), `xhigh`, `max` |
| Claude Opus 4.8 | `claude-opus-4-8` | 2× | `off`, `low`, `medium`, `high` (default), `xhigh`, `max` |
| Claude Opus 4.8 Fast | `claude-opus-4-8-fast` | 4× | `off`, `low`, `medium`, `high` (default), `xhigh`, `max` |
| Claude Opus 4.7 | `claude-opus-4-7` | 2× | `off`, `low`, `medium`, `high` (default), `xhigh`, `max` |
| Claude Opus 4.6 | `claude-opus-4-6` | 2× | `off`, `low`, `medium`, `high` (default), `max` |
| Claude Opus 4.5 | `claude-opus-4-5-20251101` | 2× | `off` (default), `low`, `medium`, `high` |
| Claude Sonnet 5.5 | `claude-sonnet-5-5` | 0.8× | `off`, `low`, `medium`, `high` (default), `xhigh`, `max` |
| Claude Sonnet 5 | `claude-sonnet-5` | 0.8× | `off`, `low`, `medium`, `high` (default), `xhigh`, `max` |
| Claude Sonnet 4.6 | `claude-sonnet-4-6` | 1.2× | `off`, `low`, `medium`, `high` (default), `max` |
| Claude Sonnet 4.5 | `claude-sonnet-4-5-20250929` | 1.2× | `off` (default), `low`, `medium`, `high` |
| Claude Haiku 5.5 | `claude-haiku-5-5` | 0.04× | `off`, `low`, `medium` (default), `high`, `xhigh`, `max` |
| Claude Haiku 4.5 | `claude-haiku-4-5-20251001` | 0.4× | `off` (default), `low`, `medium`, `high` |

<Note>
<sup>\*</sup> Anthropic requires all Mythos-class models comply with 30 day data retention for trust and safety, please see Anthropic's [data retention practices for Mythos-class models](https://support.claude.com/en/articles/15425996-data-retention-practices-for-mythos-class-models). Organization admins opt in to the Anthropic Data Retention Policy in model access settings.
</Note>

## <span className="provider-heading"><img src="/docs-assets/images/model-icons/openai.svg" alt="OpenAI logo" className="provider-icon openai-icon" />OpenAI</span>

| Model | Model ID | Multiplier | Reasoning |
| --- | --- | --- | --- |
| GPT-6.1 Sol | `gpt-6.1-sol` | 0.8× | `low`, `medium` (default), `high`, `xhigh`, `max` |
| GPT-6.1 Sol Ultra Fast | `gpt-6.1-sol-ultrafast` | 4.8× | `low`, `medium` (default), `high`, `xhigh`, `max` |
| GPT-6 Astra | `gpt-6-astra` | 4× | `low`, `medium` (default), `high`, `xhigh`, `max` |
| GPT-6 Sol | `gpt-6-sol` | 0.8× | `none`, `low`, `medium` (default), `high`, `xhigh`, `max` |
| GPT-6 Sol Fast | `gpt-6-sol-fast` | 1.6× | `none`, `low`, `medium` (default), `high`, `xhigh`, `max` |
| GPT-6 Luna | `gpt-6-luna` | 0.04× | `none`, `low`, `medium` (default), `high`, `xhigh`, `max` |
| GPT-6 Luna Fast | `gpt-6-luna-fast` | 0.08× | `none`, `low`, `medium` (default), `high`, `xhigh`, `max` |
| GPT-5.6 Sol<sup>†</sup> | `gpt-5.6-sol` | 1.6× | `none`, `low`, `medium` (default), `high`, `xhigh`, `max` |
| GPT-5.6 Sol Fast<sup>†</sup> | `gpt-5.6-sol-fast` | 3.2× | `none`, `low`, `medium` (default), `high`, `xhigh`, `max` |
| GPT-5.6 Terra | `gpt-5.6-terra` | 0.8× | `none`, `low`, `medium` (default), `high`, `xhigh`, `max` |
| GPT-5.6 Luna | `gpt-5.6-luna` | 0.08× | `none`, `low`, `medium` (default), `high`, `xhigh`, `max` |
| GPT-5.5 | `gpt-5.5` | 2× | `low`, `medium` (default), `high`, `xhigh` |
| GPT-5.5 Fast | `gpt-5.5-fast` | 5× | `low`, `medium` (default), `high`, `xhigh` |
| GPT-5.5 Pro | `gpt-5.5-pro` | 12× | `medium` (default), `high`, `xhigh` |
| GPT-5.4 | `gpt-5.4` | 1× | `low`, `medium` (default), `high`, `xhigh` |
| GPT-5.4 Fast | `gpt-5.4-fast` | 2× | `low`, `medium` (default), `high`, `xhigh` |
| GPT-5.4 Mini | `gpt-5.4-mini` | 0.3× | `low`, `medium`, `high` (default), `xhigh` |
| GPT-5.4 Mini Fast | `gpt-5.4-mini-fast` | 0.6× | `low`, `medium`, `high` (default), `xhigh` |
| GPT-5.3-Codex | `gpt-5.3-codex` | 0.7× | `low`, `medium` (default), `high`, `xhigh` |
| GPT-5.3-Codex Fast | `gpt-5.3-codex-fast` | 1.4× | `low`, `medium` (default), `high`, `xhigh` |
| GPT-5.2 | `gpt-5.2` | 0.7× | `off`, `low` (default), `medium`, `high`, `xhigh` |

<Note>
<sup>†</sup> Promotional pricing. GPT-5.6 Sol bills at 1.6× through November 22, 2026, then returns to 2×.

<sup>†</sup> Promotional pricing. GPT-5.6 Sol Fast bills at 3.2× through November 22, 2026, then returns to 4×.
</Note>

## <span className="provider-heading"><img src="/docs-assets/images/model-icons/google.ico" alt="Google logo" className="provider-icon" />Google</span>

| Model | Model ID | Multiplier | Reasoning |
| --- | --- | --- | --- |
| Gemini 3.1 Pro | `gemini-3.1-pro-preview` | 0.8× | `low`, `medium`, `high` (default) |
| Gemini 3.8 Flash<sup>†</sup> | `gemini-3.8-flash` | 0.3× | `low`, `medium`, `high` (default) |
| Gemini 3.7 Flash<sup>†</sup> | `gemini-3.7-flash` | 0.3× | `low`, `medium`, `high` (default) |
| Gemini 3.6 Flash | `gemini-3.6-flash` | 0.6× | `low`, `medium`, `high` (default) |
| Gemini 3.5 Flash | `gemini-3.5-flash` | 0.6× | `minimal`, `low`, `medium`, `high` (default) |
| Gemini 3 Flash | `gemini-3-flash-preview` | 0.2× | `minimal`, `low`, `medium`, `high` (default) |

<Note>
<sup>†</sup> Promotional pricing. Gemini 3.8 Flash bills at 0.3× through January 1, 2027, then returns to 0.6×.

<sup>†</sup> Promotional pricing. Gemini 3.7 Flash bills at 0.3× through January 1, 2027, then returns to 0.6×.
</Note>

## <span className="provider-heading"><img src="/docs-assets/images/model-icons/xai.svg" alt="xAI logo" className="provider-icon" />xAI</span>

| Model | Model ID | Multiplier | Reasoning |
| --- | --- | --- | --- |
| Grok 4.7 | `grok-4.7` | 0.8× | `low`, `medium`, `high` (default), `xhigh` |
| Grok 4.6 | `grok-4.6` | 0.8× | `low`, `medium`, `high` (default), `xhigh` |
| Grok 4.5 | `grok-4.5` | 0.8× | `low`, `medium`, `high` (default) |

## <span className="provider-heading"><img src="/brand-guidelines/logos/rotor-black.svg" alt="Factory logo" className="provider-icon" />Droid Core (Open Models)</span>

| Model | Model ID | Multiplier | Reasoning |
| --- | --- | --- | --- |
| Inkling | `inkling` | 0.4× | `off`, `minimal`, `low`, `medium`, `high` (default), `xhigh`, `max` |
| Mistral Large 4 | `mistral-large-4` | 0.544× | `off`, `high` (default) |
| Mistral Medium 3.5 | `mistral-medium-3.5` | 0.6× | `off`, `high` (default) |
| GLM-5.3-Flash | `glm-5.3-flash` | 0.06× | `low`, `high` (default), `max` |
| GLM-5.3 | `glm-5.3` | 0.56× | `low`, `high`, `max` (default) |
| GLM-5.2 | `glm-5.2` | 0.56× | `off`, `high` (default), `max` |
| GLM-5.2 Fast | `glm-5.2-fast` | 0.84× | `off`, `high` (default), `max` |
| Kimi K3 | `kimi-k3` | 1.2× | `off`, `low`, `high` (default), `max` |
| Qwen3.8 Max | `qwen3.8-max` | 0.8× | `low`, `medium`, `xhigh` (default) |
| Nemotron 3.5 Super | `nemotron-3.5-super` | 0.08× | `off`, `high` (default) |
| Nemotron 3 Ultra | `nemotron-3-ultra` | 0.24× | `off`, `high` (default) |
| DeepSeek V4.1 Flash 1017 | `deepseek-v4.1-flash-1017` | 0.12× | `off`, `low`, `high` (default), `max` |
| DeepSeek V4.1 Flash | `deepseek-v4.1-flash` | 0.12× | `off`, `low`, `high` (default), `max` |
| DeepSeek V4 Flash 0731<sup>‡</sup> | `deepseek-v4-flash-0731` | 0.176× | `off`, `low`, `high` (default), `max` |
| DeepSeek V4 Pro<sup>‡</sup> | `deepseek-v4-pro` | 0.528× | `off`, `low`, `high` (default), `max` |
| MiniMax M3.1 Flash | `minimax-m3.1-flash` | 0.06× | `low`, `medium`, `high`, `xhigh`, `max` (default) |
| MiniMax M3 | `minimax-m3` | 0.12× | `high` (default) |
| Muse Spark 1.2 | `muse-spark-1.2` | 0.5× | `low`, `medium`, `high` (default), `xhigh` |
| Beam 1.0 | `beam-1.0` | 0.52× | `low`, `medium`, `high` (default), `xhigh`, `max` |
| MiniMax M2.7<sup>‡</sup> | `minimax-m2.7` | 0.12× | `high` (default) |
| Kimi K2.5<sup>‡</sup> | `kimi-k2.5` | 0.25× | `off`, `high` (default) |
| GLM-5.1<sup>‡</sup> | `glm-5.1` | 0.55× | `off`, `high` (default) |

<Note>
<sup>‡</sup> Deprecated. DeepSeek V4 Flash 0731, DeepSeek V4 Pro, MiniMax M2.7, Kimi K2.5 and GLM-5.1 remain available for now and will be removed in a future release.
</Note>

## Custom models

Configure custom models through [Custom Models (BYOK)](/model-independence/byok).

<RelatedLinks>
  <RelatedLink href="/model-independence/byok" title="Custom Models (BYOK)">
    Configure custom model endpoints and bring your own API keys.
  </RelatedLink>
  <RelatedLink href="/model-independence/factory-router" title="Factory Router">
    Let Factory route requests across providers automatically.
  </RelatedLink>
</RelatedLinks>
