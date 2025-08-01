use derive_more::{From, Into};
use owo_colors::{OwoColorize, XtermColors};
use std::borrow::Cow;
use std::io::{self, Write};

/// `Event`s are produced automatically by using [`Metro`],
/// but can also be created and used manually.
///
/// An `Event` specifies an action and is used when rendering
/// the metro lines graph.
///
/// [`Metro`]: struct.Metro.html
///
/// # Example
///
/// ```no_run
/// use metro::Event;
///
/// let events = [
///     Event::station(0, "Station 1"),
///     Event::station(0, "Station 2"),
///     Event::station(0, "Station 3"),
///     Event::SplitTrack(0, 1),
///     Event::station(1, "Station 4"),
///     Event::SplitTrack(1, 2),
///     Event::station(1, "Station 5"),
///     Event::station(2, "Station 6"),
///     Event::station(0, "Station 7"),
///     Event::station(1, "Station 8"),
///     Event::station(2, "Station 9"),
///     Event::SplitTrack(2, 3),
///     Event::SplitTrack(3, 4),
///     Event::station(5, "Station 10 (Detached)"),
///     Event::JoinTrack(4, 0),
///     Event::station(3, "Station 11"),
///     Event::StopTrack(1),
///     Event::station(0, "Station 12"),
///     Event::station(2, "Station 13"),
///     Event::station(3, "Station 14"),
///     Event::JoinTrack(3, 0),
///     Event::station(2, "Station 15"),
///     Event::StopTrack(2),
///     Event::station(0, "Station 16"),
/// ];
///
/// let string = metro::to_string(&events).unwrap();
///
/// println!("{}", string);
/// ```
///
/// This will output the following:
///
/// ```text
/// * Station 1
/// * Station 2
/// * Station 3
/// |\
/// | * Station 4
/// | |\
/// | * | Station 5
/// | | * Station 6
/// * | | Station 7
/// | * | Station 8
/// | | * Station 9
/// | | |\
/// | | | |\
/// | | | | | Station 10 (Detached)
/// | |_|_|/
/// |/| | |
/// | | | * Station 11
/// | " | |
/// |  / /
/// * | | Station 12
/// | * | Station 13
/// | | * Station 14
/// | |/
/// |/|
/// | * Station 15
/// | "
/// * Station 16
/// ```
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

impl<'a> Event<'a> {
    /// *[See `Event::Station` for more information.][`Event::Station`]*
    ///
    /// [`Event::Station`]: enum.Event.html#variant.Station
    #[inline]
    pub fn station<S: Into<Cow<'a, str>>>(track_id: TrackId, text: S) -> Self {
        Self::Station(track_id, text.into())
    }
}

#[derive(PartialEq, Eq, Hash, From, Into, Debug, Clone, Copy)]
/// An ID referencing a `Track`
pub struct TrackId(usize);

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
pub fn to_writer<W: Write>(w: &mut W, events: &[Event]) -> io::Result<()> {
    let mut tracks = vec![0.into()];
    let widest_track = events
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

    for event in events {
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
                tracks.retain(|t| t != stopped);
                for track_id in tracks.iter() {
                    write!(
                        w,
                        "{}",
                        if track_id == stopped { "╧" } else { "│" }.color(to_color(*track_id))
                    )?;
                }
                writeln!(w)?;
            }
            Event::Station(target_id, cow) => {
                for (i, line) in cow.lines().enumerate() {
                    for track_id in tracks.iter() {
                        write!(
                            w,
                            "{}",
                            if i == 0 && track_id == target_id {
                                "║"
                            } else {
                                "│"
                            }
                            .color(to_color(*track_id))
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
                    write!(w, "{}", "│".color(to_color(*track_id)))?;
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
                                write!(w, "{}", "└┐".color(to_color(*track_id)))?;
                            } else {
                                write!(w, "{}", "│".color(to_color(*track_id)))?;
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
                            "┐"
                        } else if track_id == parent {
                            "├"
                        } else {
                            "│"
                        }
                        .color(to_color(*track_id))
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
                                "├"
                            } else {
                                "┤"
                            }
                            .color(to_color(*track_id))
                        )?;
                    } else if i == child_position {
                        write!(
                            w,
                            "{}",
                            if child_position > target_position {
                                "┘"
                            } else {
                                "└"
                            }
                            .color(to_color(*child))
                        )?;
                    } else if i > min_position && i < max_position {
                        write!(w, "{}", "─".color(to_color(*child)))?;
                    } else {
                        write!(w, "{}", "│".color(to_color(*track_id)))?;
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
                            write!(w, "{}", "┌┘".color(to_color(*track_id)))?;
                        } else {
                            write!(w, "{}", "│".color(to_color(*track_id)))?;
                        }
                    }
                    writeln!(w)?;
                }
            }
            Event::NoEvent => {
                for track_id in tracks.iter() {
                    write!(w, "{}", "│".color(to_color(*track_id)))?;
                }

                writeln!(w)?;
            }
        }
    }

    Ok(())
}

fn to_color(i: TrackId) -> XtermColors {
    XtermColors::from((((i.0 + 1) ^ 93) % 255) as u8)
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
pub fn to_vec(events: &[Event]) -> io::Result<Vec<u8>> {
    let mut vec = Vec::new();
    to_writer(&mut vec, events)?;
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
pub fn to_string(events: &[Event]) -> io::Result<String> {
    let vec = to_vec(events)?;
    // Metro only writes `str`s and `String`s to the `vec`
    // which are always valid UTF-8, so this is safe.
    #[allow(unsafe_code)]
    unsafe {
        Ok(String::from_utf8_unchecked(vec))
    }
}
