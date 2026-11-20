//! The named OAuth 2 tokens of one configuration in one collection. An index entry (`<key>`)
//! lists them and names the one in use; each token has its own entry (`<key>:<id>`), so no
//! entry grows past what a platform keychain holds. Older versions kept a single token at
//! `<key>`; it is moved into the list the first time it is read.

use serde::{Deserialize, Serialize};
use wbl_auth::Token;
use wbl_core::auth::OAuth2Config;

use crate::error::EngineError;
use crate::secrets::Secrets;

/// The keychain "environment" slot that holds tokens.
const SLOT: &str = "oauth";
/// Longest token name, in characters.
pub const MAX_NAME_CHARS: usize = 100;
const DEFAULT_PREFIX: &str = "Token ";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    pub name: String,
    pub obtained_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Index {
    pub active: Option<String>,
    /// Oldest first. Required, so that an older single token never reads as an index.
    pub tokens: Vec<Entry>,
}

impl Index {
    fn has(&self, id: &str) -> bool {
        self.tokens.iter().any(|e| e.id == id)
    }

    fn require(&self, id: &str) -> Result<(), EngineError> {
        if self.has(id) {
            Ok(())
        } else {
            Err(EngineError::new("not_found", "That token is gone"))
        }
    }

    /// The index with `entry` added and in use.
    pub fn with(&self, entry: Entry) -> Index {
        Index {
            active: Some(entry.id.clone()),
            tokens: self.tokens.iter().cloned().chain([entry]).collect(),
        }
    }

    pub fn renamed(&self, id: &str, name: &str) -> Result<Index, EngineError> {
        self.require(id)?;
        let tokens = self
            .tokens
            .iter()
            .map(|e| {
                if e.id == id {
                    Entry {
                        name: name.to_owned(),
                        ..e.clone()
                    }
                } else {
                    e.clone()
                }
            })
            .collect();
        Ok(Index {
            tokens,
            ..self.clone()
        })
    }

    pub fn selected(&self, id: &str) -> Result<Index, EngineError> {
        self.require(id)?;
        Ok(Index {
            active: Some(id.to_owned()),
            ..self.clone()
        })
    }

    /// Without `id`; removing the token in use leaves none in use.
    pub fn without(&self, id: &str) -> Result<Index, EngineError> {
        self.require(id)?;
        Ok(Index {
            active: self.active.clone().filter(|a| a != id),
            tokens: self.tokens.iter().filter(|e| e.id != id).cloned().collect(),
        })
    }

    /// `Token N` with the smallest N no token is called.
    pub fn default_name(&self) -> String {
        (1..)
            .map(|n| format!("{DEFAULT_PREFIX}{n}"))
            .find(|name| self.tokens.iter().all(|e| &e.name != name))
            .unwrap_or_else(|| DEFAULT_PREFIX.trim().to_owned())
    }

    /// An id no token has, from the time it was obtained.
    pub fn new_id(&self, now_ms: i64) -> String {
        (now_ms..)
            .map(|n| format!("{n:x}"))
            .find(|id| !self.has(id))
            .unwrap_or_default()
    }
}

/// A name as typed: trimmed, not empty, not too long.
pub fn clean_name(name: &str) -> Result<String, EngineError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(EngineError::new("invalid_argument", "A token needs a name"));
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(EngineError::new(
            "invalid_argument",
            format!("A token name has at most {MAX_NAME_CHARS} characters"),
        ));
    }
    Ok(name.to_owned())
}

/// What the frontend sees of a token: never the token itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct OAuthToken {
    pub id: String,
    pub name: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub obtained_at_ms: i64,
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub expires_at_ms: Option<i64>,
    pub scope: Option<String>,
    pub can_refresh: bool,
    pub active: bool,
}

/// The tokens of `config` in the collection `collection`.
pub struct Tokens<'a> {
    secrets: &'a Secrets,
    collection: &'a str,
    key: String,
}

fn to_json<T: Serialize>(value: &T) -> Result<String, EngineError> {
    serde_json::to_string(value).map_err(|e| EngineError::internal(e.to_string()))
}

impl<'a> Tokens<'a> {
    pub fn new(secrets: &'a Secrets, collection: &'a str, config: &OAuth2Config) -> Self {
        Self {
            secrets,
            collection,
            key: config.cache_key(),
        }
    }

    fn token_key(&self, id: &str) -> String {
        format!("{}:{id}", self.key)
    }

    fn save_index(&self, index: &Index) -> Result<(), EngineError> {
        self.secrets
            .set(self.collection, SLOT, &self.key, &to_json(index)?)
    }

    fn save_token(&self, id: &str, token: &Token) -> Result<(), EngineError> {
        self.secrets
            .set(self.collection, SLOT, &self.token_key(id), &to_json(token)?)
    }

    /// The index; an older single token becomes the first of the list.
    pub fn index(&self) -> Result<Index, EngineError> {
        let Some(text) = self.secrets.get(self.collection, SLOT, &self.key)? else {
            return Ok(Index::default());
        };
        if let Ok(index) = serde_json::from_str::<Index>(&text) {
            return Ok(index);
        }
        // An unreadable entry is as good as none: get a new token.
        let Ok(token) = serde_json::from_str::<Token>(&text) else {
            return Ok(Index::default());
        };
        self.add(&Index::default(), None, &token, wbl_auth::now_ms())
    }

    pub fn get(&self, id: &str) -> Result<Option<Token>, EngineError> {
        let text = self
            .secrets
            .get(self.collection, SLOT, &self.token_key(id))?;
        // An unreadable token is as good as none.
        Ok(text.and_then(|t| serde_json::from_str(&t).ok()))
    }

    /// The token in use, with its id.
    pub fn active(&self) -> Result<Option<(String, Token)>, EngineError> {
        let Some(id) = self.index()?.active else {
            return Ok(None);
        };
        Ok(self.get(&id)?.map(|token| (id, token)))
    }

    /// Keeps `token` under `name` (a default when `None`) and uses it. Returns the new index.
    pub fn add(
        &self,
        index: &Index,
        name: Option<&str>,
        token: &Token,
        now_ms: i64,
    ) -> Result<Index, EngineError> {
        let name = match name.map(str::trim).filter(|n| !n.is_empty()) {
            Some(n) => clean_name(n)?,
            None => index.default_name(),
        };
        let entry = Entry {
            id: index.new_id(now_ms),
            name,
            obtained_at_ms: now_ms,
        };
        self.save_token(&entry.id, token)?;
        let next = index.with(entry);
        self.save_index(&next)?;
        Ok(next)
    }

    /// Replaces a token's value (after a refresh), keeping its name.
    pub fn replace(&self, id: &str, token: &Token) -> Result<(), EngineError> {
        self.save_token(id, token)
    }

    pub fn rename(&self, id: &str, name: &str) -> Result<(), EngineError> {
        let next = self.index()?.renamed(id, &clean_name(name)?)?;
        self.save_index(&next)
    }

    pub fn select(&self, id: &str) -> Result<(), EngineError> {
        let next = self.index()?.selected(id)?;
        self.save_index(&next)
    }

    pub fn remove(&self, id: &str) -> Result<(), EngineError> {
        let next = self.index()?.without(id)?;
        self.secrets
            .clear(self.collection, SLOT, &self.token_key(id))?;
        self.save_index(&next)
    }

    /// Every token that is still in the keychain, oldest first.
    pub fn list(&self) -> Result<Vec<OAuthToken>, EngineError> {
        let index = self.index()?;
        let shown = |entry: &Entry, token: Token| OAuthToken {
            id: entry.id.clone(),
            name: entry.name.clone(),
            obtained_at_ms: entry.obtained_at_ms,
            expires_at_ms: token.expires_at_ms,
            scope: token.scope,
            can_refresh: token.refresh_token.is_some(),
            active: index.active.as_deref() == Some(entry.id.as_str()),
        };
        index
            .tokens
            .iter()
            .filter_map(|entry| {
                self.get(&entry.id)
                    .transpose()
                    .map(|token| token.map(|t| shown(entry, t)))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
