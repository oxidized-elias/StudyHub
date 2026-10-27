use crate::*;

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Provider {
    pub id: i64,
    pub label: String,
    pub kind: String,
    pub base_url: String,
    pub model: String,
    pub api_key: String,
    pub context: u32,
}
impl Provider {
    pub fn validate(&self) -> anyhow::Result<()> {
        ensure!(
            ["ollama", "openai", "claude", "gemini", "compatible"].contains(&self.kind.as_str()),
            "Choose a supported provider"
        );
        ensure!(
            !self.model.trim().is_empty() && !self.label.trim().is_empty(),
            "Model and profile name are required"
        );
        let u = reqwest::Url::parse(&self.base_url)?;
        ensure!(
            ["http", "https"].contains(&u.scheme()) && u.host_str().is_some(),
            "Endpoint must be an HTTP(S) URL, including remote Ollama hosts"
        );
        ensure!(
            u.username().is_empty()
                && u.password().is_none()
                && u.query().is_none()
                && u.fragment().is_none(),
            "Use the API key field rather than credentials or query parameters in the endpoint"
        );
        if !["ollama", "compatible"].contains(&self.kind.as_str()) {
            ensure!(
                !self.api_key.is_empty(),
                "This cloud provider needs an API key"
            );
        }
        ensure!(
            (2048..=2_000_000).contains(&self.context),
            "Context must be between 2048 and 2000000 tokens"
        );
        Ok(())
    }
    pub fn public(&self) -> Value {
        json!({"id":self.id,"label":self.label,"kind":self.kind,"base_url":self.base_url,"model":self.model,"context":self.context,"api_key_set":!self.api_key.is_empty()})
    }
}

pub fn request(
    p: &Provider,
    system: &str,
    user: &str,
    json_mode: bool,
) -> anyhow::Result<(String, Value, Vec<(String, String)>)> {
    p.validate()?;
    let base = p.base_url.trim_end_matches('/');
    let mut headers = vec![];
    let pair = match p.kind.as_str() {
        "ollama" => {
            let mut b = json!({"model":p.model,"stream":false,"messages":[{"role":"system","content":system},{"role":"user","content":user}],"options":{"num_ctx":p.context}});
            if json_mode {
                b["format"] = json!("json");
            }
            if !p.api_key.is_empty() {
                headers.push(("authorization".into(), format!("Bearer {}", p.api_key)));
            }
            (format!("{base}/api/chat"), b)
        }
        "openai" => {
            headers.push(("authorization".into(), format!("Bearer {}", p.api_key)));
            let mut b = json!({"model":p.model,"instructions":system,"input":user,"store":false});
            if json_mode {
                b["text"] = json!({"format":{"type":"json_object"}});
            }
            (format!("{base}/responses"), b)
        }
        "claude" => {
            headers.push(("x-api-key".into(), p.api_key.clone()));
            headers.push(("anthropic-version".into(), "2023-06-01".into()));
            (
                format!("{base}/messages"),
                json!({"model":p.model,"max_tokens":8192,"system":system,"messages":[{"role":"user","content":user}]}),
            )
        }
        "gemini" => {
            headers.push(("x-goog-api-key".into(), p.api_key.clone()));
            let mut b = json!({"systemInstruction":{"parts":[{"text":system}]},"contents":[{"role":"user","parts":[{"text":user}]}]});
            if json_mode {
                b["generationConfig"] = json!({"responseMimeType":"application/json"});
            }
            let model = p.model.strip_prefix("models/").unwrap_or(&p.model);
            ensure!(
                !model.contains('/') && !model.contains('?') && !model.contains('#'),
                "Enter a Gemini model ID"
            );
            (format!("{base}/models/{model}:generateContent"), b)
        }
        _ => {
            if !p.api_key.is_empty() {
                headers.push(("authorization".into(), format!("Bearer {}", p.api_key)));
            }
            let mut b = json!({"model":p.model,"messages":[{"role":"system","content":system},{"role":"user","content":user}]});
            if json_mode {
                b["response_format"] = json!({"type":"json_object"});
            }
            (format!("{base}/chat/completions"), b)
        }
    };
    Ok((pair.0, pair.1, headers))
}

pub fn response(kind: &str, v: &Value) -> anyhow::Result<String> {
    let text = match kind {
        "ollama" => v["message"]["content"].as_str().unwrap_or("").to_string(),
        "openai" => v["output"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|m| m["content"].as_array().into_iter().flatten())
            .filter_map(|c| c["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        "claude" => v["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|c| c["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        "gemini" => v["candidates"][0]["content"]["parts"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|c| c["thought"] != true)
            .filter_map(|c| c["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        _ => v["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .into(),
    };
    ensure!(
        !text.trim().is_empty(),
        "Provider returned no text; check model capabilities and provider response"
    );
    Ok(text)
}

pub async fn call(
    a: &App,
    p: &Provider,
    system: &str,
    user: &str,
    json_mode: bool,
) -> AResult<String> {
    call_logged(a, p, system, user, json_mode, runtime::Scope::default()).await
}
pub async fn call_logged(
    a: &App,
    p: &Provider,
    system: &str,
    user: &str,
    json_mode: bool,
    scope: runtime::Scope,
) -> AResult<String> {
    let started = std::time::Instant::now();
    let endpoint = reqwest::Url::parse(&p.base_url)
        .map(|url| url.origin().ascii_serialization())
        .unwrap_or_else(|_| "Invalid endpoint URL".into());
    runtime::log(
        a,
        scope,
        "request_started",
        "Preparing model request",
        json!({"provider_id":p.id,"provider":p.kind,"model":p.model,"endpoint":endpoint,"context":p.context}),
    );
    let result: AResult<String> = async {
        ensure!(user.chars().count()+system.chars().count()<(p.context as usize).saturating_mul(3),"Selected material exceeds this provider's context budget. Select fewer sources or increase the configured context.");
        let (url,body,headers)=request(p,system,user,json_mode)?;
        let mut req=a.http.post(url).json(&body);for(k,v)in headers{req=req.header(k,v);}
        runtime::log(a,scope,"provider_wait","Waiting for the model server; this may include loading weights, prompt processing and generation",json!({}));
        let response=req.send().await.map_err(|e|anyhow!("{}",if e.is_timeout(){"Model request timed out. Check model loading, available memory and server logs.".to_string()}else{format!("Cannot reach model server: {}",e.without_url())}))?;
        let status=response.status();
        runtime::log(a,scope,"provider_http","Model server responded",json!({"http_status":status.as_u16(),"elapsed_ms":started.elapsed().as_millis() as u64}));
        let body=response.text().await.context("Could not read model response")?;
        if !status.is_success(){let detail=runtime::provider_error(&body,&p.api_key).unwrap_or_else(||"Check the endpoint, installed model, available memory and model server logs".into());bail!("Provider returned HTTP {status}: {detail}");}
        let value:Value=serde_json::from_str(&body).context("Model server returned invalid JSON")?;
        let text=self::response(&p.kind,&value)?;
        runtime::log(a,scope,"response_received","Model response received",json!({"characters":text.chars().count(),"elapsed_ms":started.elapsed().as_millis() as u64,"load_duration_ns":value.get("load_duration"),"prompt_tokens":value.get("prompt_eval_count"),"generated_tokens":value.get("eval_count")}));
        Ok(text)
    }.await;
    if let Err(error) = &result {
        runtime::log(
            a,
            scope,
            "request_failed",
            &runtime::safe_error(&error.to_string(), &p.api_key),
            json!({"elapsed_ms":started.elapsed().as_millis() as u64}),
        );
    }
    result
}

pub fn model_names(kind: &str, value: &Value) -> AResult<Vec<String>> {
    let (key, field) = if kind == "ollama" {
        ("models", "name")
    } else {
        ("data", "id")
    };
    let rows = value[key]
        .as_array()
        .ok_or_else(|| anyhow!("Server returned no model list"))?;
    let mut names = rows
        .iter()
        .filter_map(|v| v[field].as_str())
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovery_lists_server_models_without_duplicates() {
        assert_eq!(
            model_names(
                "ollama",
                &json!({"models":[{"name":"qwen:9b"},{"name":"gemma:4b"},{"name":"qwen:9b"}]})
            )
            .unwrap(),
            vec!["gemma:4b", "qwen:9b"]
        );
        assert_eq!(
            model_names("compatible", &json!({"data":[{"id":"local-tutor"}]})).unwrap(),
            vec!["local-tutor"]
        );
        assert!(model_names("ollama", &json!({"error":"offline"})).is_err());
        assert!(model_names("ollama", &json!({"models":[]}))
            .unwrap()
            .is_empty());
    }
    fn p(kind: &str) -> Provider {
        Provider {
            id: 1,
            label: "Test".into(),
            kind: kind.into(),
            base_url: match kind {
                "ollama" => "http://192.168.1.42:11434",
                "gemini" => "https://generativelanguage.googleapis.com/v1beta",
                _ => "https://example.com/v1",
            }
            .into(),
            model: "test-model".into(),
            api_key: "private-key".into(),
            context: 16384,
        }
    }
    #[test]
    fn cloud_and_remote_local_requests_use_the_right_protocol() {
        for kind in ["ollama", "openai", "claude", "gemini", "compatible"] {
            let p = p(kind);
            let (url, body, headers) = request(&p, "System", "User", true).unwrap();
            assert!(!body.to_string().contains("private-key"));
            assert!(!p.public().to_string().contains("private-key"));
            match kind {
                "ollama" => {
                    assert_eq!(url, "http://192.168.1.42:11434/api/chat");
                    assert_eq!(body["stream"], false);
                    assert_eq!(body["format"], "json")
                }
                "openai" => {
                    assert!(url.ends_with("/responses"));
                    assert_eq!(body["store"], false);
                    assert_eq!(body["text"]["format"]["type"], "json_object")
                }
                "claude" => {
                    assert!(url.ends_with("/messages"));
                    assert!(headers.iter().any(|(k, _)| k == "x-api-key"))
                }
                "gemini" => {
                    assert!(url.ends_with("/models/test-model:generateContent"));
                    assert_eq!(
                        body["generationConfig"]["responseMimeType"],
                        "application/json"
                    )
                }
                _ => assert!(url.ends_with("/chat/completions")),
            }
        }
    }
    #[test]
    fn response_adapters_keep_all_text_blocks() {
        assert_eq!(response("openai",&json!({"output":[{"type":"reasoning"},{"content":[{"type":"output_text","text":"one"},{"type":"output_text","text":"two"}]}]})).unwrap(),"one\ntwo");
        assert_eq!(response("claude",&json!({"content":[{"type":"thinking","thinking":"hidden"},{"type":"text","text":"answer"}]})).unwrap(),"answer");
        assert_eq!(response("gemini",&json!({"candidates":[{"content":{"parts":[{"thought":true,"text":"hidden"},{"text":"answer"}]}}]})).unwrap(),"answer");
        assert!(response("ollama", &json!({})).is_err())
    }
    #[test]
    fn credentials_in_urls_are_rejected() {
        let mut p = p("ollama");
        p.base_url = "http://user:pass@host:11434".into();
        assert!(p.validate().is_err());
        p.base_url = "file:///etc/passwd".into();
        assert!(p.validate().is_err())
    }
}
