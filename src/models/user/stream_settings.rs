use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamSettings {
    #[serde(default)]
    pub scoreboard: ScoreboardSettings,
    #[serde(default)]
    pub scenes: SceneSettings,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreboardSettings {
    pub enabled: bool,
    pub home_name: String,
    pub away_name: String,
    pub home_abbr: String,
    pub away_abbr: String,
    pub home_color: String,
    pub away_color: String,
    pub timer_mode: String,
    pub timer_duration: u32,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneSettings {
    pub choices: Vec<Option<SceneChoice>>,
    pub picture_in_picture: Option<PictureInPicture>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", rename_all_fields = "camelCase")]
pub enum SceneChoice {
    PictureInPicture,
    Video { video_id: String },
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PictureInPicture {
    pub main_id: String,
    pub inset_id: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamSettingsPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scoreboard: Option<ScoreboardSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenes: Option<SceneSettings>,
}

impl Default for StreamSettings {
    fn default() -> Self {
        Self {
            scoreboard: ScoreboardSettings::default(),
            scenes: SceneSettings::default(),
        }
    }
}

impl Default for ScoreboardSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            home_name: "Home".into(),
            away_name: "Away".into(),
            home_abbr: "HOM".into(),
            away_abbr: "AWY".into(),
            home_color: "#2463eb".into(),
            away_color: "#e33446".into(),
            timer_mode: "up".into(),
            timer_duration: 1800,
        }
    }
}

impl Default for SceneSettings {
    fn default() -> Self {
        Self {
            choices: vec![Some(SceneChoice::PictureInPicture), None, None],
            picture_in_picture: None,
        }
    }
}

impl StreamSettingsPatch {
    pub fn validate(&self) -> bool {
        self.scoreboard.as_ref().map_or(true, |score| {
            score.home_name.chars().count() <= 40
                && score.away_name.chars().count() <= 40
                && score.home_abbr.chars().count() <= 3
                && score.away_abbr.chars().count() <= 3
                && valid_color(&score.home_color)
                && valid_color(&score.away_color)
                && ["up", "down"].contains(&score.timer_mode.as_str())
                && score.timer_duration <= 86_400
        }) && self.scenes.as_ref().map_or(true, |scenes| {
            scenes.choices.len() == 3
                && scenes.choices.iter().flatten().all(valid_choice)
                && scenes.picture_in_picture.as_ref().map_or(true, |pip| {
                    valid_slot(&pip.main_id)
                        && valid_slot(&pip.inset_id)
                        && pip.main_id != pip.inset_id
                        && [pip.x, pip.y, pip.width, pip.height].iter().all(|n| n.is_finite() && (0.0..=1.0).contains(n))
                        && pip.width > 0.0
                        && pip.height > 0.0
                })
        }) && (self.scoreboard.is_some() || self.scenes.is_some())
    }
}

fn valid_color(color: &str) -> bool {
    color.len() == 7
        && color.starts_with('#')
        && color[1..].chars().all(|c| c.is_ascii_hexdigit())
}

fn valid_slot(slot: &str) -> bool {
    matches!(slot, "slot-0" | "slot-1" | "slot-2" | "slot-3")
}

fn valid_choice(choice: &SceneChoice) -> bool {
    match choice {
        SceneChoice::PictureInPicture => true,
        SceneChoice::Video { video_id } => valid_slot(video_id),
    }
}

impl StreamSettings {
    pub fn from_json(text: Option<String>) -> Result<Self, serde_json::Error> {
        match text {
            Some(text) => {
                let mut saved: Self = serde_json::from_str(&text)?;
                if saved.scenes.choices.len() != 3 { saved.scenes.choices = Self::default().scenes.choices; }
                Ok(saved)
            }
            None => Ok(Self::default()),
        }
    }
}
