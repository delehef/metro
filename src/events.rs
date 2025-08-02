use compact_str::CompactString;
use derive_more::{From, Into};
use owo_colors::{OwoColorize, XtermColors};
use std::borrow::Cow;
use std::io::{self, Write};

#[derive(Clone, Copy)]
enum Rail {
    Straight,
    Horizontal,
    Station,
    Ground,
    ShiftRight,
    ShiftLeft,
    TopRight,
    BottomRight,
    BottomtLeft,
    SplitRight,
    SplitLeft,
}

pub struct RenderingSettings {
    splat: usize,
    color: bool,
    rounded: bool,
}
impl Default for RenderingSettings {
    fn default() -> Self {
        Self {
            splat: 5,
            color: true,
            rounded: false,
        }
    }
}
impl RenderingSettings {
    pub fn splat(mut self, splat_factor: usize) -> Self {
        self.splat = splat_factor;
        self
    }

    pub fn color(mut self, colored: bool) -> Self {
        self.color = colored;
        self
    }

    fn colorize<S: AsRef<str>>(&self, s: S, i: &TrackId) -> CompactString {
        if self.color {
            let color = XtermColors::from((((i.0 + 1) ^ 93) % 255) as u8);
            s.as_ref().color(color).to_string().into()
        } else {
            s.as_ref().into()
        }
    }

    fn rail_to_str(&self, rail: Rail) -> CompactString {
        use std::fmt::Write;

        let mut r = CompactString::with_capacity(self.splat + 2);
        match rail {
            Rail::Straight => write!(r, "│{}", " ".repeat(self.splat)),
            Rail::Horizontal => write!(r, "{}", "─".repeat(self.splat + 1)),
            Rail::Station => write!(r, "╪{}", " ".repeat(self.splat)),
            Rail::Ground => write!(r, "┷{}", " ".repeat(self.splat)),
            Rail::ShiftRight => write!(r, "└{}┐{}", "─".repeat(self.splat), " ".repeat(self.splat)),
            Rail::ShiftLeft => write!(r, "┌{}┘", "─".repeat(self.splat)),
            Rail::TopRight => write!(r, "{}┐{}", "─".repeat(self.splat), " ".repeat(self.splat)),
            Rail::BottomRight => write!(r, "{}┘{}", "─".repeat(self.splat), " ".repeat(self.splat)),
            Rail::BottomtLeft => write!(r, "└{}", "─".repeat(self.splat)),
            Rail::SplitRight => write!(r, "├"),
            Rail::SplitLeft => write!(r, "{}┤", "─".repeat(self.splat)),
        }
        .unwrap();
        r
    }
}

trait RenderStr {
    fn render(&self, s: &RenderingSettings, i: &TrackId) -> CompactString;
}
impl RenderStr for Rail {
    fn render(&self, s: &RenderingSettings, i: &TrackId) -> CompactString {
        s.colorize(s.rail_to_str(*self), i)
    }
}

#[derive(PartialEq, Eq, Hash, From, Into, Debug, Clone, Copy)]
/// An ID referencing a `Track`
pub struct TrackId(usize);

#[derive(Clone, Debug)]
pub enum Event<'a> {
    /// `StartTrack(track_id)`
    ///
    /// - If `track_id` already exists, then this event does nothing.
    ///
    /// New `track_id`s are added rightmost.
    ///
    /// ## Output Example
    ///
    /// Given 3 tracks `0, 1, 2` then `StartTrack(4)` would render as:
    ///
    /// ```text
    /// | | |
    /// | | | |
    /// ```
    StartTrack(TrackId),

    /// `StartTracks(track_ids)`
    ///
    /// - If a `track_id` from `track_ids` already exists, then it is ignored.
    /// - If all `track_ids` already exists, then this event does nothing.
    ///
    /// New `track_id`s are added rightmost.
    ///
    /// ## Output Example
    ///
    /// Given 3 tracks `0, 1, 2` then `StartTracks(&[4, 5])` would render as:
    ///
    /// ```text
    /// | | |
    /// | | | | |
    /// ```
    StartTracks(&'a [TrackId]),

    /// `StopTrack(track_id)`
    ///
    /// - If `track_id` does not exist, then this event does nothing.
    ///
    /// All rails to the right of `track_id`, are pulled to the left.
    ///
    /// ## Output Example
    ///
    /// Given 3 tracks `0, 1, 2` then `StopTrack(1)` would render as:
    ///
    /// ```text
    /// | | |
    /// | " |
    /// |  /
    /// | |
    /// ```
    StopTrack(TrackId),

    /// `Station(track_id, text)`
    ///
    /// - If the `track_id` does not exist, then `text` is still
    /// rendered, just not tied to any track.
    ///
    /// ## Output Example
    ///
    /// Given 3 tracks `0, 1, 2` then `Station(1, "Hello World")` would render as:
    ///
    /// ```text
    /// | | |
    /// | * | Hello World
    /// | | |
    /// ```
    ///
    /// Text with multiple lines is also allowed.
    /// Given 3 tracks `0, 1, 2` then `Station(1, "Hello\nWorld")` would render as:
    ///
    /// ```text
    /// | | |
    /// | * | Hello
    /// | | | World
    /// | | |
    /// ```
    ///
    /// If the `track_id` does not exist, then no rail is highlighted.
    /// Thus `Station(10, "Hello World")` would render as:
    ///
    /// ```text
    /// | | |
    /// | | | Hello World
    /// | | |
    /// ```
    Station(TrackId, Cow<'a, str>),

    /// `SplitTrack(from_track_id, new_track_id)`
    ///
    /// Creates a new track diverging from `from_track_id` to the right.
    /// All rails to the right of `from_track_id`, are pushed to the
    /// right to make space for the new track.
    ///
    /// - If `from_track_id` does not exist, then this event is the
    /// same as `StartTrack(new_track_id)`.
    /// - If `new_track_id` already exists, then this event does nothing.
    ///
    /// ## Output Example
    ///
    /// Given 3 tracks `0, 1, 2` then `SplitTrack(1, 4)` would render as:
    ///
    /// ```text
    /// | | |
    /// | |\ \
    /// | | | |
    /// ```
    SplitTrack(TrackId, TrackId),

    /// `JoinTrack(from_track_id, to_track_id)`
    ///
    /// Joins `from_track_id` and `to_track_id`
    /// resulting in the `from_track_id` being removed.
    ///
    /// The rails are joined towards the leftmost rail.
    ///
    /// - If `from_track_id` does not exist, then this event does nothing.
    /// - If `to_track_id` does not exist, then it turns into `StopTrack(from_track_id)`.
    /// - If `from_track_id` and `to_track_id` are the same, then it turns into `StopTrack(from_track_id)`
    ///
    /// The track ID (`from_track_id`) can be reused for
    /// a new track after this event.
    ///
    /// ## Output Example
    ///
    /// Given 3 tracks `0, 1, 2` then `JoinTrack(1, 0)` would render as:
    ///
    /// ```text
    /// | | |
    /// |/ /
    /// | |
    /// ```
    ///
    /// Given 6 tracks `0, 1, 2, 3, 4, 5` then `JoinTrack(4, 0)` would render as:
    ///
    /// ```text
    /// | | | | | |
    /// | |_|_|/ /
    /// |/| | | |
    /// | | | | |
    /// ```
    JoinTrack(TrackId, TrackId),

    /// `NoEvent` produces one row of rails.
    ///
    /// ## Output Example
    ///
    /// Given 3 tracks `0, 1, 2` then `NoEvent` would render as:
    ///
    /// ```text
    /// | | |
    /// ```
    NoEvent,
}

#[derive(Default)]
pub struct Metro<'a> {
    events: Vec<Event<'a>>,
    rdr: RenderingSettings,
}
impl<'a> Metro<'a> {
    pub fn with_settings(rdr: RenderingSettings) -> Self {
        Self {
            rdr,
            ..Default::default()
        }
    }
    pub fn push(&mut self, event: Event<'a>) {
        self.events.push(event);
    }
    /// Write `&[`[`Event`]`]` to [`<W: io::Write>`].
    /// Defines a default track with `track_id` of `0`.
    ///
    /// *[See also `Metro::to_writer`.][`Metro::to_writer`]*
    ///
    /// *See also [`to_string`] and [`to_vec`].*
    ///
    /// [`to_vec`]: fn.to_vec.html
    /// [`to_string`]: fn.to_string.html
    ///
    /// [`Event`]: enum.Event.html
    ///
    /// [`Metro::to_writer`]: struct.Metro.html#method.to_writer
    ///
    /// [`<W: io::Write>`]: https://doc.rust-lang.org/stable/std/io/trait.Write.html
    pub fn to_writer<W: Write>(&self, w: &mut W) -> io::Result<()> {
        let mut tracks = vec![0.into()];
        let widest_track = self
            .events
            .iter()
            .fold((1, 1), |(current, max), e| {
                let current = match e {
                    Event::StartTrack(_) => current + 1,
                    Event::StartTracks(track_ids) => current + track_ids.len(),
                    Event::StopTrack(_) => current - 1,
                    Event::SplitTrack(_, _) => current + 1,
                    Event::JoinTrack(_, _) => current - 1,
                    _ => current,
                };
                (current, max.max(current))
            })
            .1;

        for event in self.events.iter() {
            match event {
                Event::StartTrack(track_id) => {
                    assert!(!tracks.contains(track_id));
                    tracks.push(*track_id);
                }
                Event::StartTracks(track_ids) => {
                    for track_id in track_ids.iter() {
                        assert!(!tracks.contains(track_id));
                        tracks.push(*track_id);
                    }
                }
                Event::StopTrack(stopped) => {
                    assert!(tracks.contains(stopped));
                    for track_id in tracks.iter() {
                        write!(
                            w,
                            "{}",
                            if track_id == stopped {
                                Rail::Ground
                            } else {
                                Rail::Ground
                                // Rail::Straight
                            }
                            .render(&self.rdr, track_id)
                        )?;
                    }
                    writeln!(w)?;
                    tracks.retain(|t| t != stopped);
                }
                Event::Station(target_id, cow) => {
                    for (i, line) in cow.lines().enumerate() {
                        for track_id in tracks.iter() {
                            write!(
                                w,
                                "{}",
                                if i == 0 && track_id == target_id {
                                    Rail::Station
                                } else {
                                    Rail::Straight
                                }
                                .render(&self.rdr, track_id)
                            )?;
                        }
                        write!(
                            w,
                            "{line:>pad$}",
                            pad = line.len() + widest_track - tracks.len() + 3
                        )?;
                        writeln!(w)?;
                    }
                    for track_id in tracks.iter() {
                        write!(w, "{}", Rail::Straight.render(&self.rdr, track_id))?;
                    }
                    writeln!(w)?;
                }
                Event::SplitTrack(parent, child) => {
                    let parent_position = tracks
                        .iter()
                        .position(|t| t == parent)
                        .expect(&format!("no parent {parent:?} found in {tracks:?}"));
                    if tracks.len() > 1 {
                        for l_i in 0..(tracks.len() - parent_position) {
                            for (i, track_id) in tracks.iter().enumerate() {
                                let ii = tracks.len() - i;
                                if ii == l_i {
                                    write!(w, "{}", Rail::ShiftRight.render(&self.rdr, track_id))?;
                                } else {
                                    write!(w, "{}", Rail::Straight.render(&self.rdr, track_id))?;
                                }
                            }
                            writeln!(w)?;
                        }
                    }
                    tracks.insert(parent_position + 1, *child);
                    for track_id in tracks.iter() {
                        write!(
                            w,
                            "{}",
                            if track_id == child {
                                Rail::TopRight
                            } else if track_id == parent {
                                Rail::SplitRight
                            } else {
                                Rail::Straight
                            }
                            .render(&self.rdr, track_id)
                        )?;
                    }
                    writeln!(w)?;
                }
                Event::JoinTrack(child, target) => {
                    let target_position = tracks.iter().position(|t| t == target).unwrap();
                    let child_position = tracks
                        .iter()
                        .position(|t| t == child)
                        .expect(&format!("child {child:?} not found in {tracks:?}"));
                    let min_position = target_position.min(child_position);
                    let max_position = target_position.max(child_position);
                    for (i, track_id) in tracks.iter().enumerate() {
                        if i == target_position {
                            write!(
                                w,
                                "{}",
                                if child_position > target_position {
                                    Rail::SplitRight
                                } else {
                                    Rail::SplitLeft
                                }
                                .render(&self.rdr, track_id)
                            )?;
                        } else if i == child_position {
                            write!(
                                w,
                                "{}",
                                if child_position > target_position {
                                    Rail::BottomRight
                                } else {
                                    Rail::BottomtLeft
                                }
                                .render(&self.rdr, child)
                            )?;
                        } else if i > min_position && i < max_position {
                            write!(w, "{}", Rail::Horizontal.render(&self.rdr, child))?;
                        } else {
                            write!(w, "{}", Rail::Straight.render(&self.rdr, track_id))?;
                        }
                    }
                    writeln!(w)?;
                    tracks.retain(|t| t != child);
                    for i in if child_position > target_position {
                        max_position
                    } else {
                        min_position + 1
                    }..tracks.len()
                    {
                        for (j, track_id) in tracks.iter().enumerate() {
                            if j == i && j != 0 {
                                write!(w, "{}", Rail::ShiftLeft.render(&self.rdr, track_id))?;
                            } else {
                                write!(w, "{}", Rail::Straight.render(&self.rdr, track_id))?;
                            }
                        }
                        writeln!(w)?;
                    }
                }
                Event::NoEvent => {
                    for track_id in tracks.iter() {
                        write!(w, "{}", Rail::Straight.render(&self.rdr, track_id))?;
                    }

                    writeln!(w)?;
                }
            }
        }

        Ok(())
    }

    /// Write `&[`[`Event`]`]` to [`Vec<u8>`].
    /// Defines a default track with `track_id` of `0`.
    ///
    /// *[See also `Metro::to_vec`.][`Metro::to_vec`]*
    ///
    /// *See also [`to_string`] and [`to_writer`].*
    ///
    /// [`to_writer`]: fn.to_writer.html
    /// [`to_string`]: fn.to_string.html
    ///
    /// [`Event`]: enum.Event.html
    ///
    /// [`Metro::to_vec`]: struct.Metro.html#method.to_vec
    ///
    /// [`Vec<u8>`]: https://doc.rust-lang.org/stable/std/vec/struct.Vec.html
    pub fn to_vec(&self) -> io::Result<Vec<u8>> {
        let mut vec = Vec::new();
        self.to_writer(&mut vec)?;
        Ok(vec)
    }

    /// Write `&[`[`Event`]`]` to [`String`].
    /// Defines a default track with `track_id` of `0`.
    ///
    /// *[See also `Metro::to_string`.][`Metro::to_string`]*
    ///
    /// *See also [`to_vec`] and [`to_writer`].*
    ///
    /// [`to_writer`]: fn.to_writer.html
    /// [`to_vec`]: fn.to_vec.html
    ///
    /// [`Event`]: enum.Event.html
    ///
    /// [`Metro::to_string`]: struct.Metro.html#method.to_string
    ///
    /// [`String`]: https://doc.rust-lang.org/stable/std/string/struct.String.html
    pub fn to_string(&self) -> io::Result<String> {
        let vec = self.to_vec()?;
        // Metro only writes `str`s and `String`s to the `vec`
        // which are always valid UTF-8, so this is safe.
        #[allow(unsafe_code)]
        unsafe {
            Ok(String::from_utf8_unchecked(vec))
        }
    }
}
