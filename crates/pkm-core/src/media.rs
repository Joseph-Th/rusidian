//! Media types and embed providers for rich block content.
//!
//! Provides strongly-typed enums for media formats and external embed sources,
//! ensuring AI agents cannot generate malformed markdown syntax.

use serde::{Deserialize, Serialize};

/// The type of media in a Media block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
    /// Raster or vector image (PNG, JPEG, SVG, etc.)
    Image,
    /// Audio file (MP3, WAV, FLAC, etc.)
    Audio,
    /// Video file (MP4, WebM, etc.)
    Video,
    /// PDF document
    Pdf,
}

impl MediaType {
    /// Extension hint for the media type. Used when storing or referencing files.
    pub fn default_extension(&self) -> &'static str {
        match self {
            MediaType::Image => ".png",
            MediaType::Audio => ".mp3",
            MediaType::Video => ".mp4",
            MediaType::Pdf => ".pdf",
        }
    }

    /// MIME type for this media.
    pub fn mime_type(&self) -> &'static str {
        match self {
            MediaType::Image => "image/*",
            MediaType::Audio => "audio/*",
            MediaType::Video => "video/*",
            MediaType::Pdf => "application/pdf",
        }
    }
}

/// External service provider for iFrame embeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbedProvider {
    /// YouTube video embed (youtube.com)
    YouTube,
    /// Twitter/X tweet embed (twitter.com / x.com)
    Twitter,
    /// Google Drive document embed (docs.google.com)
    GoogleDrive,
    /// Generic iframe embed for other providers
    Generic,
}

impl EmbedProvider {
    /// Get the domain for this provider.
    pub fn domain(&self) -> &'static str {
        match self {
            EmbedProvider::YouTube => "youtube.com",
            EmbedProvider::Twitter => "twitter.com",
            EmbedProvider::GoogleDrive => "docs.google.com",
            EmbedProvider::Generic => "embed.example.com",
        }
    }

    /// Detect provider from URL. Returns Generic for unrecognized URLs.
    /// Handles both HTTP and HTTPS URLs.
    pub fn from_url(url: &str) -> Self {
        let lower = url.to_lowercase();
        if lower.contains("youtube.com") || lower.contains("youtu.be") {
            EmbedProvider::YouTube
        } else if lower.contains("twitter.com") || lower.contains("x.com") {
            EmbedProvider::Twitter
        } else if lower.contains("docs.google.com") || lower.contains("drive.google.com") {
            EmbedProvider::GoogleDrive
        } else {
            EmbedProvider::Generic
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_type_has_sensible_defaults() {
        assert_eq!(MediaType::Image.default_extension(), ".png");
        assert_eq!(MediaType::Audio.default_extension(), ".mp3");
        assert_eq!(MediaType::Video.default_extension(), ".mp4");
        assert_eq!(MediaType::Pdf.default_extension(), ".pdf");
    }

    #[test]
    fn embed_provider_detection_from_url() {
        assert_eq!(
            EmbedProvider::from_url("https://youtube.com/watch?v=dQw4w9WgXcQ"),
            EmbedProvider::YouTube
        );
        assert_eq!(
            EmbedProvider::from_url("https://youtu.be/dQw4w9WgXcQ"),
            EmbedProvider::YouTube
        );
        assert_eq!(
            EmbedProvider::from_url("https://twitter.com/example/status/123"),
            EmbedProvider::Twitter
        );
        assert_eq!(
            EmbedProvider::from_url("https://x.com/example/status/123"),
            EmbedProvider::Twitter
        );
        assert_eq!(
            EmbedProvider::from_url("https://docs.google.com/document/d/abc"),
            EmbedProvider::GoogleDrive
        );
        assert_eq!(
            EmbedProvider::from_url("https://drive.google.com/file/d/abc"),
            EmbedProvider::GoogleDrive
        );
        assert_eq!(
            EmbedProvider::from_url("https://example.com/page"),
            EmbedProvider::Generic
        );
    }

    #[test]
    fn media_type_mime_types_are_correct() {
        assert_eq!(MediaType::Image.mime_type(), "image/*");
        assert_eq!(MediaType::Audio.mime_type(), "audio/*");
        assert_eq!(MediaType::Video.mime_type(), "video/*");
        assert_eq!(MediaType::Pdf.mime_type(), "application/pdf");
    }

    #[test]
    fn embed_provider_from_url_is_case_insensitive() {
        assert_eq!(
            EmbedProvider::from_url("HTTPS://YOUTUBE.COM/WATCH"),
            EmbedProvider::YouTube
        );
        assert_eq!(
            EmbedProvider::from_url("HTTP://TWITTER.COM/STATUS"),
            EmbedProvider::Twitter
        );
        assert_eq!(
            EmbedProvider::from_url("https://DOCS.GOOGLE.COM/document"),
            EmbedProvider::GoogleDrive
        );
    }

    #[test]
    fn embed_provider_serializes_correctly() {
        let providers = vec![
            EmbedProvider::YouTube,
            EmbedProvider::Twitter,
            EmbedProvider::GoogleDrive,
            EmbedProvider::Generic,
        ];

        for provider in providers {
            let json = serde_json::to_string(&provider).expect("serialize");
            let back: EmbedProvider = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, provider);
        }
    }

    #[test]
    fn media_type_serializes_correctly() {
        let types = vec![
            MediaType::Image,
            MediaType::Audio,
            MediaType::Video,
            MediaType::Pdf,
        ];

        for media_type in types {
            let json = serde_json::to_string(&media_type).expect("serialize");
            let back: MediaType = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, media_type);
        }
    }
}
