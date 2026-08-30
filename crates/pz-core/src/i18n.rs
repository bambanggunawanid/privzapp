//! Locales and the translation catalog (ADR-0014).
//!
//! English is the source language and the canonical, unprefixed URL
//! space (`/tool/merge-pdf`). Every other locale lives under its code
//! (`/id/tool/merge-pdf`), which is what makes `hreflang` and separate
//! indexing possible — the whole point of doing this for SEO.
//!
//! UI strings are keyed by their **English text**, gettext-style. That
//! keeps call sites readable (`t(loc, "Choose a PDF to edit")` instead
//! of an invented key), and means a missing translation degrades to
//! English rather than to a broken key. The trade is that editing an
//! English string silently orphans its translation, so
//! `catalog_is_sane` guards the shape of the table and the UI tests
//! check the strings that matter.

use crate::{tool_by_slug, ToolMeta};

/// Every language the app ships. Adding one is: a variant, its rows in
/// the three tables below, and nothing else — routing, `hreflang`, the
/// sitemap and the switcher all iterate `Locale::ALL`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Locale {
    En,
    Id,
}

impl Locale {
    pub const ALL: &'static [Locale] = &[Locale::En, Locale::Id];

    /// BCP 47 code, used for `<html lang>`, `hreflang` and URLs.
    pub fn code(self) -> &'static str {
        match self {
            Locale::En => "en",
            Locale::Id => "id",
        }
    }

    /// How the language names itself, for the switcher.
    pub fn endonym(self) -> &'static str {
        match self {
            Locale::En => "English",
            Locale::Id => "Bahasa Indonesia",
        }
    }

    /// URL prefix. English is unprefixed so existing links keep working
    /// and the primary language keeps the shortest, strongest URLs.
    pub fn prefix(self) -> &'static str {
        match self {
            Locale::En => "",
            Locale::Id => "/id",
        }
    }

    pub fn is_default(self) -> bool {
        self == Locale::En
    }
}

/// Parses a URL segment. Deliberately strict: this is used as a routed
/// path segment, so it must reject real route names like "tool",
/// "privacy" and "support" or `/tool/x` would be read as locale "tool".
/// English is intentionally NOT parsed — `/en/...` would be a duplicate
/// of the canonical URL and split its ranking.
impl std::str::FromStr for Locale {
    // Not `()`: the router's `FromRouteSegment` requires the error to be
    // `Display` so it can report which segment failed to parse.
    type Err = UnknownLocale;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "id" => Ok(Locale::Id),
            _ => Err(UnknownLocale),
        }
    }
}

/// Returned when a URL segment is not a supported language code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownLocale;

impl std::fmt::Display for UnknownLocale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("not a supported language code")
    }
}

impl std::error::Error for UnknownLocale {}

impl std::fmt::Display for Locale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}

/// Translate a UI string. `en` is returned unchanged when it has no
/// entry, so an untranslated string shows in English rather than
/// breaking the page.
pub fn t(locale: Locale, en: &str) -> &str {
    if locale.is_default() {
        return en;
    }
    UI_ID
        .iter()
        .find(|(key, _)| *key == en)
        .map(|(_, translated)| *translated)
        .unwrap_or(en)
}

/// A tool's display name in `locale`.
pub fn tool_name(meta: &ToolMeta, locale: Locale) -> &'static str {
    localized(TOOL_TEXT_ID, meta.slug, locale)
        .map(|(name, _)| name)
        .unwrap_or(meta.name)
}

/// A tool's one-line description in `locale`.
pub fn tool_tagline(meta: &ToolMeta, locale: Locale) -> &'static str {
    localized(TOOL_TEXT_ID, meta.slug, locale)
        .map(|(_, tagline)| tagline)
        .unwrap_or(meta.tagline)
}

/// Same, by slug — convenient for the prerenderer.
pub fn tool_name_by_slug(slug: &str, locale: Locale) -> Option<&'static str> {
    tool_by_slug(slug).map(|m| tool_name(m, locale))
}

fn localized(
    table: &'static [(&'static str, &'static str, &'static str)],
    slug: &str,
    locale: Locale,
) -> Option<(&'static str, &'static str)> {
    if locale.is_default() {
        return None;
    }
    table
        .iter()
        .find(|(s, _, _)| *s == slug)
        .map(|(_, name, tagline)| (*name, *tagline))
}

/// Title/description for the non-tool pages ("home", "privacy",
/// "support"). English is passed in and returned unchanged, so the
/// source copy stays where it is written (in seo-gen) and only the
/// translation lives here.
pub fn site_page_seo(
    locale: Locale,
    key: &str,
    en_title: &'static str,
    en_desc: &'static str,
) -> (&'static str, &'static str) {
    if locale.is_default() {
        return (en_title, en_desc);
    }
    PAGE_SEO_ID
        .iter()
        .find(|(k, _, _)| *k == key)
        .map(|(_, t, d)| (*t, *d))
        .unwrap_or((en_title, en_desc))
}

include!("i18n_id.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TOOLS;

    #[test]
    fn locale_parsing_never_swallows_a_route() {
        // If any of these parsed as a locale, /tool/x would be routed as
        // locale "tool" and every tool page would 404.
        for not_a_locale in ["tool", "privacy", "support", "en", "", "assets"] {
            assert!(
                not_a_locale.parse::<Locale>().is_err(),
                "{not_a_locale:?} must not parse as a locale"
            );
        }
        assert_eq!("id".parse::<Locale>().unwrap(), Locale::Id);
    }

    #[test]
    fn english_is_unprefixed_and_others_are_not() {
        assert_eq!(Locale::En.prefix(), "");
        for loc in Locale::ALL.iter().filter(|l| !l.is_default()) {
            assert_eq!(loc.prefix(), format!("/{}", loc.code()));
        }
    }

    #[test]
    fn every_tool_is_translated() {
        for tool in TOOLS {
            let name = tool_name(tool, Locale::Id);
            let tagline = tool_tagline(tool, Locale::Id);
            assert!(
                TOOL_TEXT_ID.iter().any(|(s, _, _)| *s == tool.slug),
                "tool \"{}\" has no Indonesian name/tagline in TOOL_TEXT_ID",
                tool.slug
            );
            assert!(!name.is_empty() && !tagline.is_empty());
        }
    }

    #[test]
    fn catalog_is_sane() {
        for (key, value) in UI_ID {
            assert!(!key.is_empty(), "empty catalog key");
            assert!(!value.is_empty(), "empty translation for {key:?}");
            assert_ne!(key, value, "untranslated catalog entry: {key:?}");
        }
        let mut keys: Vec<&str> = UI_ID.iter().map(|(k, _)| *k).collect();
        keys.sort_unstable();
        let before = keys.len();
        keys.dedup();
        assert_eq!(before, keys.len(), "duplicate key in UI_ID");

        let mut slugs: Vec<&str> = TOOL_TEXT_ID.iter().map(|(s, _, _)| *s).collect();
        slugs.sort_unstable();
        let before = slugs.len();
        slugs.dedup();
        assert_eq!(before, slugs.len(), "duplicate slug in TOOL_TEXT_ID");
    }

    #[test]
    fn translation_falls_back_to_english() {
        assert_eq!(t(Locale::En, "anything at all"), "anything at all");
        assert_eq!(
            t(Locale::Id, "a string nobody has translated"),
            "a string nobody has translated"
        );
    }
}
