//! Arrow rendering library for list view
//! Arrows are _not_ rendered by maintaining state line-by-line, but are initially given a depth,
//! allowing arrows for a given line to be rendered completely independently. This allows 
//! drawing arrows for a given range, without having to also compute arrows for previous lines.

use std::cmp::{max, min};
use std::fmt;

// use mcd::disassembly::BasicBlock;

/// Represents a range of (global) addresses.
#[derive(Debug)]
struct AddressRange {
    start: usize,
    end: usize,
}

impl AddressRange {
    /// Check if range intersects with other range, range will be flipped if necessary
    fn intersects(&self, other: &AddressRange) -> bool {
        let self_min = min(self.start, self.end);
        let self_max = max(self.start, self.end);
        let other_min = min(other.start, other.end);
        let other_max = max(other.start, other.end);
        self_min <= other_max && other_min <= self_max
    }

    fn new(start: usize, end: usize) -> AddressRange {
        assert!(start != end);
        Self { start, end }
    }

    fn last_addr(&self) -> usize {
        if self.end > self.start {
            self.end
        } else {
            self.start
        }
    }

    fn first_addr(&self) -> usize {
        if self.end > self.start {
            self.start
        } else {
            self.end
        }
    }
}

/// Used for rendering in color
#[derive(Debug)]
pub enum ArrowType {
    Always,
    IfTrue,
    IfFalse,
}

#[derive(Debug)]
struct Arrow {
    range: AddressRange,
    depth: u8,
    arrowtype: ArrowType,
}

impl Arrow {
    /// If self intersects with other (test arrow), return depth of self
    fn depth_if_intersect(&self, other: &AddressRange) -> Option<u8> {
        match self.range.intersects(other) {
            true => Some(self.depth),
            false => None,
        }
    }
}

#[derive(Debug)]
pub struct Arrows {
    arrows: Vec<Arrow>,
    max_depth: u8,
}

impl Arrows {
    pub fn new() -> Self {
        Arrows {
            arrows: Vec::new(),
            max_depth: 0,
        }
    }
    
    /// Add arrows within a function, given indices
    pub fn add_arrow(&mut self, start: usize, end: usize, arrowtype: ArrowType) {
        // Ignore fallthrough arrows
        if end > start && end - start == 1{
            return
        }
        // choose least depth that is free - not necesssarily a globally optimal solution?
        let range = AddressRange::new(start, end);
        let intersect_depths = self
            .arrows
            .iter()
            .filter_map(|arrow| arrow.depth_if_intersect(&range))
            .collect::<Vec<_>>();

        let mut depth = 0;
        while intersect_depths.contains(&depth) {
            depth += 1;
        }
        self.max_depth = self.max_depth.max(depth);
        self.arrows.push(Arrow {
            range,
            depth,
            arrowtype,
        });
    }

    fn max_depth(&self) -> Option<u8> {
        (!self.arrows.is_empty()).then_some(self.max_depth)
    }

    fn space_needed(&self) -> u8 {
        match self.max_depth() {
            None => 0,
            Some(depth) => depth + 1, // depth with one extra for gutter
        }
    }

    /// Nudge before is useful for blocks labels, where the same address spans multiple lines. Since we only want arrows starting/ending in one of these
    /// set nudge_before in all calls except for the final call of that address where you DO want arrows to start/end
    pub fn render_at_addr(&self, f: &mut fmt::Formatter<'_>, addr: usize, nudge_before: bool) -> fmt::Result {
        let mut arrow_exit = false;
        let mut arrow_enter = false;

        #[derive(Debug)]
        enum LineTypes {
            None,
            Starting,
            Stopping,
            Through,
        }

        let depths: Vec<_> = self
            .arrows
            .iter()
            .filter(|arrow| arrow.range.first_addr() <= addr && addr <= arrow.range.last_addr())
            .map(|arrow| {
                let ty = if nudge_before {
                    // if thru, do not render if it is furst arrow, and arrow is FULLY below
                    if arrow.range.first_addr() == addr
                        && arrow.range.last_addr() > arrow.range.first_addr()
                    {
                        LineTypes::None
                    } else {
                        LineTypes::Through
                    }
                } else {
                    if arrow.range.start == addr {
                        arrow_exit = true;
                    }
                    if arrow.range.end == addr {
                        arrow_enter = true;
                    }

                    if addr == arrow.range.last_addr() {
                        LineTypes::Stopping
                    } else if addr == arrow.range.first_addr() {
                        LineTypes::Starting
                    } else {
                        LineTypes::Through
                    }
                };
                (arrow.depth, ty)
            })
            .collect();


        let mut draw_remaining = false;

        // rev means lower depths printed further right (closer to the disassembly)
        for i in (0..self.space_needed()).rev() {
            let found = depths.iter().find(|(key, _status)| *key ==  i);
            if let Some((_found_key, linetype)) = found {
                match linetype {
                    LineTypes::Starting => {
                        if !draw_remaining {
                            write!(f, "╭")?
                        } else {
                            write!(f, "┬")?
                        }
                    }
                    LineTypes::Stopping => {
                        if !draw_remaining {
                            write!(f, "╰")?
                        } else {
                            write!(f, "┴")?
                        }
                    }
                    LineTypes::Through => write!(f, "│")?,
                    LineTypes::None => write!(f, " ")?,
                }
                if matches!(linetype, LineTypes::Starting)
                    || matches!(linetype, LineTypes::Stopping)
                {
                    draw_remaining = true
                }
            } else if draw_remaining {
                write!(f, "─")?
            } else {
                write!(f, " ")?
            };
        }

        match (arrow_enter, arrow_exit) {
            (true, true) => write!(f, "■")?,
            (true, false) => write!(f, "▶")?,
            (false, true) => write!(f, "─")?,
            (false, false) => write!(f, " ")?,
        }
        write!(f, " ")?; // Arrow occupies two character widths, so pad accordingly
        Ok(())
    }
}