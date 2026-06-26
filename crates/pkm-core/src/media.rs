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
    pub fn from_url(url: &str) -> Self {
        if url.contains("youtube.com") || url.contains("youtu.be") {
            EmbedProvider::YouTube
        } else if url.contains("twitter.com") || url.contains("x.com") {
            EmbedProvider::Twitter
        } else if url.contains("docs.google.com") {
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
            EmbedProvider::from_url("https://twitter.com/example/status/123"),
            EmbedProvider::Twitter
        );
        assert_eq!(
            EmbedProvider::from_url("https://docs.google.com/document/d/abc"),
            EmbedProvider::GoogleDrive
        );
        assert_eq!(
            EmbedProvider::from_url("https://example.com/page"),
            EmbedProvider::Generic
        );
    }
}
