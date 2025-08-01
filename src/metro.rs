use std::borrow::Cow;
use std::cell::RefCell;
use std::fmt;
use std::io::{self, Write};
use std::mem;
use std::rc::Rc;

use crate::events::{to_string, to_vec, to_writer, Event};
use crate::TrackId;

type RcMetro<'a> = Rc<RefCell<MetroState<'a>>>;

/// The `Metro` struct is essentially a more friendly way and
/// builder for creating an [`Event`] stream (`&[`[`Event`]`]`).
///
/// The *edge cases* of using [`Event`]s manually and creating
/// a `&[`[`Event`]`]`, is not possible using `Metro` and [`Track`].
///
/// [`Track`]: struct.Track.html
/// [`Event`]: enum.Event.html
///
/// # Example
///
/// ```no_run
/// use metro::Metro;
///
/// let mut metro = Metro::new();
///
/// let mut track1 = metro.new_track();
/// track1.add_station("Station 1");
/// track1.add_station("Station 2");
/// track1.add_station("Station 3");
///
/// let mut track2 = track1.split();
/// track2.add_station("Station 4");
///
/// let mut track3 = track2.split();
/// track2.add_station("Station 5");
/// track3.add_station("Station 6");
///
/// track1.add_station("Station 7");
/// track2.add_station("Station 8");
/// track3.add_station("Station 9");
///
/// let mut track4 = track3.split();
/// let track5 = track4.split();
///
/// metro.add_station("Station 10 (Detached)");
///
/// track5.join(&track1);
///
/// track4.add_station("Station 11");
///
/// track2.stop();
///
/// track1.add_station("Station 12");
/// track3.add_station("Station 13");
/// track4.add_station("Station 14");
///
/// track4.join(&track1);
///
/// track3.add_station("Station 15");
///
/// track3.stop();
///
/// track1.add_station("Station 16");
///
/// let string = metro.to_string().unwrap();
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
///
/// # Internally
///
/// `Metro` and [`Track`] uses an [`Rc`]`<`[`RefCell`]`<MetroState>>`
/// internally to keep track of everything, such that `Metro` and
/// [`Track`] can supply a more friendly way of constructing graphs.
///
/// [`Rc`]: https://doc.rust-lang.org/stable/std/rc/struct.Rc.html
/// [`RefCell`]: https://doc.rust-lang.org/stable/std/cell/struct.RefCell.html
///
/// If you are able to trigger a [`BorrowError`] or
/// [`BorrowMutError`], then please submit an issue on
/// the [issue tracker], with a snippet to reproduce
/// the error.
///
/// [`BorrowError`]: https://doc.rust-lang.org/stable/std/cell/struct.BorrowError.html
/// [`BorrowMutError`]: https://doc.rust-lang.org/stable/std/cell/struct.BorrowMutError.html
///
/// Additionally, if you have a need for [`Arc`] then feel free to submit
/// an issue on the [issue tracker] with your use case.
///
/// [issue tracker]: https://github.com/vallentin/metro/issues
/// [`Arc`]: https://doc.rust-lang.org/std/sync/struct.Arc.html
#[allow(missing_debug_implementations)]
pub struct Metro<'a> {
    state: RcMetro<'a>,
}

impl<'a> Metro<'a> {
    /// Create a new `Metro`.
    #[allow(clippy::new_without_default)]
    #[inline]
    pub fn new() -> Self {
        Self {
            state: Rc::new(RefCell::new(MetroState::new())),
        }
    }

    // [`Metro`]: struct.Metro.html
    // [`new_track`]: struct.Metro.html#method.new_track

    /// Create a new [`Track`].
    ///
    /// To create a new [`Track`] with a specific track [`id`], then use [`new_track_with_id`].
    ///
    /// [`new_track_with_id`]: struct.Metro.html#method.new_track_with_id
    ///
    /// [`Track`]: struct.Track.html
    /// [`id`]: struct.Track.html#method.id
    ///
    /// # Panics
    ///
    /// Panics if more than [`usize`] tracks have been created.
    ///
    /// [`usize`]: https://doc.rust-lang.org/stable/std/primitive.usize.html
    ///
    /// ## Output Example
    ///
    /// Given 3 existing tracks then `new_track` would render as:
    ///
    /// ```text
    /// | | |
    /// | | | |
    /// ```
    #[inline]
    pub fn new_track(&mut self) -> Track<'a> {
        let id = self.state.borrow_mut().next_id();
        self.new_track_with_id(id.into())
    }

    /// Create a new [`Track`] with a specific track [`id`].
    ///
    /// If the track [`id`] is already in use, then this call has the same effect
    /// as calling [`get_track(id)`]`.unwrap()`.
    ///
    /// To create a new [`Track`] without a specific track [`id`], then use [`new_track`].
    ///
    /// [`new_track`]: struct.Metro.html#method.new_track
    /// [`get_track(id)`]: struct.Metro.html#method.get_track
    ///
    /// [`Track`]: struct.Track.html
    /// [`id`]: struct.Track.html#method.id
    ///
    /// # Panics
    ///
    /// Panics if more than [`usize`] tracks have been created.
    ///
    /// [`usize`]: https://doc.rust-lang.org/stable/std/primitive.usize.html
    ///
    /// ## Output Example
    ///
    /// Given 3 existing tracks then `new_track_with_id` would render as:
    ///
    /// ```text
    /// | | |
    /// | | | |
    /// ```
    #[inline]
    pub fn new_track_with_id(&mut self, track_id: TrackId) -> Track<'a> {
        MetroState::new_track(&self.state, track_id)
    }

    /// If the `track_id` exists then `Some` is returned, otherwise `None`.
    #[inline]
    pub fn get_track(&mut self, track_id: TrackId) -> Option<Track<'a>> {
        MetroState::get_track(&self.state, track_id)
    }

    /// Creates a station that is not tied to any [`Track`].
    ///
    /// See [`Track::add_station`] to create a station that is
    /// tied to a [`Track`].
    ///
    /// Note that all stations require a `track_id`, so it uses
    /// [`std::usize::MAX`] as `track_id`.
    ///
    /// [`Track::add_station`]: struct.Track.html#method.add_station
    ///
    /// [`std::usize::MAX`]: https://doc.rust-lang.org/stable/std/usize/constant.MAX.html
    ///
    /// ## Output Example
    ///
    /// Given 3 existing tracks then `add_station("Hello World")` would render as:
    ///
    /// ```text
    /// | | |
    /// | | | Hello World
    /// | | |
    /// ```
    ///
    /// Text with multiple lines is also allowed.
    /// Given 3 existing tracks then `add_station("Hello\nWorld")` would render as:
    ///
    /// ```text
    /// | | |
    /// | | | Hello
    /// | | | World
    /// | | |
    /// ```
    #[inline]
    pub fn add_station<S: Into<Cow<'a, str>>>(&mut self, text: S) {
        MetroState::add_event(&self.state, Event::station(std::usize::MAX.into(), text));
    }

    /// *[See `to_writer`.][`to_writer`]*
    ///
    /// [`to_writer`]: fn.to_writer.html
    #[inline]
    pub fn to_writer<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let state = self.state.borrow();
        to_writer(writer, &state.events)
    }

    /// *[See `to_vec`.][`to_vec`]*
    ///
    /// [`to_vec`]: fn.to_vec.html
    #[inline]
    pub fn to_vec(&self) -> io::Result<Vec<u8>> {
        let state = self.state.borrow();
        to_vec(&state.events)
    }

    /// *[See `to_string`.][`to_string`]*
    ///
    /// [`to_string`]: fn.to_string.html
    #[inline]
    pub fn to_string(&self) -> io::Result<String> {
        let state = self.state.borrow();
        to_string(&state.events)
    }

    /// Returns [`Vec`]`<`[`Event`]`>` of the events currently
    /// in this `Metro`.
    ///
    /// *[See also `into_events`.][`into_events`]*
    ///
    /// *[See also `Metro::to_string`.][`to_string`]*
    ///
    /// [`into_events`]: struct.Metro.html#method.into_events
    /// [`to_string`]: struct.Metro.html#method.to_string
    /// [`Event`]: enum.Event.html
    ///
    /// [`Vec`]: https://doc.rust-lang.org/stable/std/vec/struct.Vec.html
    #[inline]
    pub fn to_events(&self) -> Vec<Event<'a>> {
        let state = self.state.borrow();
        state.events.clone()
    }

    /// Consumes `Metro` and returns its [`Vec`]`<`[`Event`]`>`
    /// of the events.
    ///
    /// *[See also `to_events`.][`to_events`]*
    ///
    /// *[See also `Metro::to_string`.][`to_string`]*
    ///
    /// [`to_events`]: struct.Metro.html#method.to_events
    /// [`to_string`]: struct.Metro.html#method.to_string
    #[inline]
    pub fn into_events(self) -> Vec<Event<'a>> {
        let mut state = self.state.borrow_mut();
        mem::replace(&mut state.events, Vec::new())
    }
}

/// The `Track` struct represents a track in the [`Metro`].
/// The `Track` struct is created with the [`new_track`] or
/// [`new_track_with_id`] on [`Metro`].
///
/// *[See `Metro` for a complete example.][`Metro`]*
///
/// [`Metro`]: struct.Metro.html
/// [`new_track`]: struct.Metro.html#method.new_track
/// [`new_track_with_id`]: struct.Metro.html#method.new_track_with_id
pub struct Track<'a> {
    state: RcMetro<'a>,
    id: TrackId,
}

impl<'a> Track<'a> {
    fn new(state: RcMetro<'a>, id: TrackId) -> Self {
        Self { state, id }
    }

    /// Returns the track id.
    pub fn id(&self) -> TrackId {
        self.id
    }

    /// Stop this `Track`, and removes it from [`Metro`].
    ///
    /// The track [`id`] can be reused after with [`new_track_with_id`].
    ///
    /// ### Dangling Tracks
    ///
    /// If other tracks exist, which has the same track [`id`], e.g. by
    /// calling `get_track` multiple times with the same track [`id`], then
    /// the other tracks become "dangling". However, if a new track is created
    /// with [`new_track_with_id`], then the other references are
    /// not dangling anymore.
    ///
    /// *[See `is_dangling` for more information.][`is_dangling`]*
    ///
    /// [`id`]: struct.Track.html#method.id
    /// [`is_dangling`]: struct.Track.html#method.is_dangling
    ///
    /// [`Metro`]: struct.Metro.html
    /// [`new_track_with_id`]: struct.Metro.html#method.new_track_with_id
    /// [`get_track`]: struct.Metro.html#method.get_track
    ///
    /// ## Output Example
    ///
    /// Given 3 existing tracks then calling `stop()` on the middle track would render as:
    ///
    /// ```text
    /// | | |
    /// | " |
    /// |  /
    /// | |
    /// ```
    #[inline]
    pub fn stop(self) {
        // Method is empty as the logic is implemented in Drop for Track
    }

    /// Creates a station that is tied to this `Track`.
    ///
    /// See [`Metro::add_station`] to create a station that is
    /// tied to a `Track`.
    ///
    /// [`Metro::add_station`]: struct.Metro.html#method.add_station
    ///
    /// ## Output Example
    ///
    /// Given 3 existing tracks then `add_station("Hello World")` on the middle track would render as:
    ///
    /// ```text
    /// | | |
    /// | * | Hello World
    /// | | |
    /// ```
    ///
    /// Text with multiple lines is also allowed.
    /// Given 3 existing tracks then `add_station("Hello\nWorld")` would render as:
    ///
    /// ```text
    /// | | |
    /// | * | Hello
    /// | | | World
    /// | | |
    /// ```
    #[inline]
    pub fn add_station<S: Into<Cow<'a, str>>>(&mut self, text: S) {
        MetroState::add_event(&self.state, Event::station(self.id, text));
    }

    /// Create a new `Track` that branches of from this track.
    ///
    /// To create a new `Track` with a specific track [`id`], then use [`new_track_with_id`].
    ///
    /// [`id`]: struct.Track.html#method.id
    /// [`new_track_with_id`]: struct.Track.html#method.split_with_id
    ///
    /// # Panics
    ///
    /// Panics if more than [`usize`] tracks have been created.
    ///
    /// [`usize`]: https://doc.rust-lang.org/stable/std/primitive.usize.html
    ///
    /// ## Output Example
    ///
    /// Given 3 existing tracks then calling `split` on the middle track would render as:
    ///
    /// ```text
    /// | | |
    /// | |\ \
    /// | | | |
    /// ```
    #[inline]
    pub fn split(&self) -> Track<'a> {
        let id = self.state.borrow_mut().next_id();
        self.split_with_id(id.into())
    }

    /// Create a new `Track` that branches of from this track.
    ///
    /// To create a new `Track` without a specific track [`id`], then use [`split`].
    ///
    /// [`id`]: struct.Track.html#method.id
    /// [`split`]: struct.Track.html#method.split
    ///
    /// # Panics
    ///
    /// Panics if more than [`usize`] tracks have been created.
    ///
    /// [`usize`]: https://doc.rust-lang.org/stable/std/primitive.usize.html
    ///
    /// ## Output Example
    ///
    /// Given 3 existing tracks then calling `split` on the middle track would render as:
    ///
    /// ```text
    /// | | |
    /// | |\ \
    /// | | | |
    /// ```
    #[inline]
    pub fn split_with_id(&self, new_track_id: TrackId) -> Track<'a> {
        MetroState::split_track(&self.state, self, new_track_id)
    }

    /// Merges `self` with `to_track`, removing `self` from
    /// the [`Metro`].
    ///
    /// [`Metro`]: struct.Metro.html
    ///
    /// ## Output Example
    ///
    /// Given 3 existing tracks, then calling join on the middle
    /// track with the leftmost track would render as:
    ///
    /// ```text
    /// | | |
    /// |/ /
    /// | |
    /// ```
    ///
    /// Given 6 existing tracks, then calling join on the second
    /// rightmost track with the leftmost track would render as:
    ///
    /// ```text
    /// | | | | | |
    /// | |_|_|/ /
    /// |/| | | |
    /// | | | | |
    /// ```
    #[inline]
    pub fn join(self, to_track: &Track) {
        MetroState::join_track(&self.state, &self, to_track);
    }

    /// Returns `true` if the `Track` has been removed from
    /// its [`Metro`].
    ///
    /// Note if a new track is created in the same [`Metro`], with
    /// the same track [`id`], then `self` is no longer dangling
    /// and the two `Track`s represent the same `Track`.
    ///
    /// [`Metro`]: struct.Metro.html
    /// [`id`]: struct.Track.html#method.id
    ///
    /// # Example
    ///
    /// ```
    /// # use metro::Metro;
    /// let mut metro = Metro::new();
    ///
    /// // Create a new track
    /// let mut track1 = metro.new_track();
    ///
    /// // Get a second reference to the same track
    /// let mut track2 = metro.get_track(track1.id()).unwrap();
    ///
    /// // They represent the same track, so `is_dangling` returns `false` for both
    /// assert!(!track1.is_dangling());
    /// assert!(!track2.is_dangling());
    ///
    /// // Stop the track
    /// track1.stop();
    /// // or
    /// // drop(track1);
    ///
    /// // Now `track2` is dangling as `track1` was stopped and they share track id
    /// assert!(track2.is_dangling());
    ///
    /// // Create a new track that uses the same track id `0`
    ///
    /// // Create a new track with the same track id as `track2`
    /// let mut track3 = metro.new_track_with_id(track2.id());
    ///
    /// // Now `track2` and `track3` represent the same track,
    /// // so `is_dangling` again returns `false` for both
    /// assert!(!track2.is_dangling());
    /// assert!(!track3.is_dangling());
    /// ```
    #[inline]
    pub fn is_dangling(&self) -> bool {
        self.state
            .borrow()
            .tracks
            .iter()
            .all(|track| track.id != self.id)
    }

    #[inline]
    fn clone_ref(&self) -> Self {
        Self {
            state: Rc::clone(&self.state),
            id: self.id,
        }
    }
}

impl<'a> Drop for Track<'a> {
    /// Drop implicitly calls [`Track::stop`].
    ///
    /// [`Track::stop`]: struct.Track.html#method.stop
    #[inline]
    fn drop(&mut self) {
        // Is `Track` still present in its `Metro`?
        let is_dangling = self
            .state
            // When `metro.tracks.remove(index)` is called, then
            // `Metro` is going to be mutably borrowed already,
            // while triggering this `Drop`.
            .try_borrow()
            .map(|metro| metro.tracks.iter().all(|track| track.id != self.id))
            // If "already mutably borrowed" when dropping,
            // then assume it is in the context of something
            // performing `tracks.remove(index)`, thus we
            // assume the `Track` is dangling and already
            // removed or being removed.
            .unwrap_or(true);

        if !is_dangling {
            MetroState::add_event(&self.state, Event::StopTrack(self.id));

            let mut state = self.state.borrow_mut();

            // Remove the `Track` from its `Metro`
            let index = state
                .tracks
                .iter()
                .position(|track| track.id == self.id)
                // Safe to use `unwrap` as `is_dangling` just verified the `Track`'s presence
                .unwrap();
            state.tracks.remove(index);
        }
    }
}

impl fmt::Debug for Track<'_> {
    #[inline]
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Track").field("id", &self.id).finish()
    }
}

struct MetroState<'a> {
    tracks: Vec<Track<'a>>,
    events: Vec<Event<'a>>,
    next_id: usize,
}

impl<'a> MetroState<'a> {
    #[inline]
    fn new() -> Self {
        Self {
            tracks: vec![],
            events: vec![],
            next_id: 0,
        }
    }

    /// Get a new track id.
    ///
    /// # Panics
    ///
    /// Panics if the `next_id` overflows [`usize`].
    ///
    /// [`usize`]: https://doc.rust-lang.org/stable/std/primitive.usize.html
    fn next_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn new_track(metro: &RcMetro<'a>, track_id: TrackId) -> Track<'a> {
        let state = metro.borrow();
        let track = state.tracks.iter().find(|track| track.id == track_id);

        if let Some(track) = track {
            track.clone_ref()
        } else {
            drop(state);

            let track = Track::new(Rc::clone(metro), track_id);
            metro.borrow_mut().tracks.push(track.clone_ref());

            MetroState::add_event(metro, Event::StartTrack(track_id));

            track
        }
    }

    fn get_track(metro: &RcMetro<'a>, track_id: TrackId) -> Option<Track<'a>> {
        let state = metro.borrow();
        let track = state.tracks.iter().find(|track| track.id == track_id);

        track.map(Track::clone_ref)
    }

    fn split_track(metro: &RcMetro<'a>, from_track: &Track, new_track_id: TrackId) -> Track<'a> {
        let state = metro.borrow();
        let new_track = state.tracks.iter().find(|track| track.id == new_track_id);

        if let Some(new_track) = new_track {
            new_track.clone_ref()
        } else {
            drop(state);

            let new_track = Track::new(Rc::clone(metro), new_track_id);
            metro.borrow_mut().tracks.push(new_track.clone_ref());

            MetroState::add_event(metro, Event::SplitTrack(from_track.id(), new_track_id));

            new_track
        }
    }

    /// The caller must consume `from_track`.
    /// The caller must not produce `Event::StopTrack`.
    #[inline]
    fn join_track(metro: &RcMetro<'a>, from_track: &Track, to_track: &Track) {
        let from_track_id = from_track.id();

        // Whether either track already stopped existing does not matter
        // as `to_string` handles rendering and resolving "edge cases".
        MetroState::add_event(metro, Event::JoinTrack(from_track_id, to_track.id()));

        let mut state = metro.borrow_mut();

        // If `from_track` `is_dangling` then it has already been removed from `tracks`
        if let Some(index) = state
            .tracks
            .iter()
            .position(|track| track.id == from_track_id)
        {
            state.tracks.remove(index);
        }
    }

    #[inline]
    fn add_event(metro: &RcMetro<'a>, event: Event<'a>) {
        metro.borrow_mut().events.push(event);
    }
}
