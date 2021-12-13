#![allow(unused)]

//! Generate the main table seen within the TUI that shows the file paths and
//! tags. This is used to iterate over Row, Cell, Spans, and Span to get the
//! styles for each individual tag as they're seen on the regular TUI. This part
//! of the module also allows for some additional features that the default
//! `Table` from `tui` doesn't have.
//!
//! This includes header alignment, highlighted marker, highlighted non-marker,
//! column spacing, etc

// Credit: idea and outline came from `kdheepak/taskwarrior-tui`
//  * Using their work to help me learn how to code a TUI

use cassowary::{
    strength::{MEDIUM, REQUIRED, WEAK},
    Expression, Solver,
    WeightedRelation::{EQ, GE, LE},
};
use std::{
    collections::{hash_set::Iter, HashMap, HashSet},
    fmt::{self, Display},
    iter::{self, Iterator},
};
use tui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Span, Text},
    widgets::{Block, StatefulWidget, Widget},
};
use unicode_segmentation::{Graphemes, UnicodeSegmentation};
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone)]
pub(crate) enum TableSelection {
    Single,
    Multiple,
}

#[derive(Debug, Clone)]
pub(crate) struct TableState {
    offset:            usize,
    current_selection: Option<usize>,
    marked:            HashSet<usize>,
    mode:              TableSelection,
}

impl Default for TableState {
    fn default() -> TableState {
        TableState {
            offset:            0,
            current_selection: Some(0),
            marked:            HashSet::new(),
            mode:              TableSelection::Single,
        }
    }
}

impl TableState {
    pub(crate) fn mode(&self) -> TableSelection {
        self.mode.clone()
    }

    pub(crate) fn multiple_selection(&mut self) {
        self.mode = TableSelection::Multiple;
    }

    pub(crate) fn single_selection(&mut self) {
        self.mode = TableSelection::Single;
    }

    pub(crate) fn current_selection(&self) -> Option<usize> {
        self.current_selection
    }

    pub(crate) fn select(&mut self, index: Option<usize>) {
        self.current_selection = index;
        if index.is_none() {
            self.offset = 0;
        }
    }

    pub(crate) fn mark(&mut self, index: Option<usize>) {
        if let Some(i) = index {
            self.marked.insert(i);
        }
    }

    pub(crate) fn unmark(&mut self, index: Option<usize>) {
        if let Some(i) = index {
            self.marked.remove(&i);
        }
    }

    pub(crate) fn toggle_mark(&mut self, index: Option<usize>) {
        if let Some(i) = index {
            if !self.marked.insert(i) {
                self.marked.remove(&i);
            }
        }
    }

    pub(crate) fn marked(&self) -> Iter<usize> {
        self.marked.iter()
    }

    pub(crate) fn clear(&mut self) {
        self.marked.drain().for_each(drop);
    }
}

// /// Holds data to be displayed in a Table widget
// #[derive(Debug, Clone)]
// pub(crate) enum ModifiedRow<D>
// where
//     D: Iterator,
//     D::Item: Display,
// {
//     Data(D),
//     StyledData(D, Style),
// }

#[allow(single_use_lifetimes)]
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Cell<'a> {
    content: Text<'a>,
    style:   Style,
}

// impl Display for Cell<'_> {
//     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//         write!(f, "{}", self.content.lines)
//     }
// }

impl Cell<'_> {
    /// Set the `Style` of this cell.
    pub(crate) fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl<'a, T> From<T> for Cell<'a>
where
    T: Into<Text<'a>>,
{
    fn from(content: T) -> Cell<'a> {
        Cell {
            content: content.into(),
            style:   Style::default(),
        }
    }
}

/// By default, a row has a height of 1 but you can change this using
/// [`Row::height`].
#[derive(Debug, Clone, PartialEq, Default)]
#[allow(single_use_lifetimes)]
pub(crate) struct Row<'a> {
    cells:         Vec<Cell<'a>>,
    height:        u16,
    style:         Style,
    bottom_margin: u16,
}

impl<'a> Row<'a> {
    /// Creates a new `Row` from an iterator where items can be converted to a
    /// `Cell`
    pub(crate) fn new<T>(cells: T) -> Self
    where
        T: IntoIterator,
        T::Item: Into<Cell<'a>>,
    {
        Self {
            height:        1,
            cells:         cells.into_iter().map(Into::into).collect(),
            style:         Style::default(),
            bottom_margin: 0,
        }
    }

    /// Set the fixed height of the [`Row`]. Any [`Cell`] whose content has more
    /// lines than this height will see its content truncated
    pub(crate) const fn height(mut self, height: u16) -> Self {
        self.height = height;
        self
    }

    /// Set the [`Style`] of the entire row. This [`Style`] can be overriden by
    /// the [`Style`] of a any individual [`Cell`] or event by their
    /// [`Text`] content
    pub(crate) const fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Set the bottom margin. By default, the bottom margin is `0`
    pub(crate) const fn bottom_margin(mut self, margin: u16) -> Self {
        self.bottom_margin = margin;
        self
    }

    /// Returns the total height of the row
    const fn total_height(&self) -> u16 {
        self.height.saturating_add(self.bottom_margin)
    }
}

/// Table module
#[derive(Debug, Clone)]
pub(crate) struct Table<'a, H> {
    /// A block to wrap the widget in
    block:                   Option<Block<'a>>,
    /// Base style for the widget
    style:                   Style,
    /// Header row for all columns
    header:                  H,
    /// Style for the header
    header_style:            Style,
    /// Alignment for the header
    header_alignment:        Alignment,
    /// Width constraints for each column
    widths:                  &'a [Constraint],
    /// Space between each column
    column_spacing:          u16,
    /// Space between the header and the rows
    header_gap:              u16,
    /// Whether selection indicator style should be used on tags
    highlight_tags:          bool,
    /// Style used to render the selected row
    highlight_style:         Style,
    /// Symbol in front of the selected row
    highlight_symbol:        Option<&'a str>,
    /// Symbol in front of the marked row
    mark_symbol:             Option<&'a str>,
    /// Symbol in front of the unmarked row
    unmark_symbol:           Option<&'a str>,
    /// Symbol in front of the marked and selected row
    mark_highlight_symbol:   Option<&'a str>,
    /// Symbol in front of the unmarked and selected row
    unmark_highlight_symbol: Option<&'a str>,
    /// Data to display in each row
    rows:                    Vec<Row<'a>>,
}

impl<H> Default for Table<'_, H>
where
    H: Iterator + Default + fmt::Debug + Clone,
{
    fn default() -> Self {
        Table {
            block:                   None,
            style:                   Style::default(),
            header:                  H::default(),
            header_style:            Style::default(),
            header_alignment:        Alignment::Left,
            widths:                  &[],
            column_spacing:          1,
            header_gap:              1,
            highlight_style:         Style::default(),
            highlight_symbol:        None,
            highlight_tags:          false,
            mark_symbol:             None,
            unmark_symbol:           None,
            mark_highlight_symbol:   None,
            unmark_highlight_symbol: None,
            rows:                    Vec::new(),
        }
    }
}

impl<'a, H> Table<'a, H>
where
    H: Iterator,
{
    pub(crate) fn new<R>(header: H, rows: R) -> Self
    where
        R: IntoIterator<Item = Row<'a>>,
    {
        Self {
            block: None,
            style: Style::default(),
            header,
            header_style: Style::default(),
            header_alignment: Alignment::Left,
            widths: &[],
            column_spacing: 1,
            header_gap: 1,
            highlight_style: Style::default(),
            highlight_symbol: None,
            highlight_tags: false,
            mark_symbol: None,
            unmark_symbol: None,
            mark_highlight_symbol: None,
            unmark_highlight_symbol: None,
            rows: rows.into_iter().collect(),
        }
    }

    /// Change/set block that is used to outline the table
    pub(crate) fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Change/set header text (below title)
    pub(crate) fn header<II>(mut self, header: II) -> Self
    where
        II: IntoIterator<Item = H::Item, IntoIter = H>,
    {
        self.header = header.into_iter();
        self
    }

    /// Change/set header display style
    pub(crate) fn header_style(mut self, style: Style) -> Self {
        self.header_style = style;
        self
    }

    /// Change/set alignment of the header
    pub(crate) fn header_alignment(mut self, alignment: Alignment) -> Self {
        self.header_alignment = alignment;
        self
    }

    /// TODO: ??
    pub(crate) fn widths(mut self, widths: &'a [Constraint]) -> Self {
        let between_0_and_100 = |&w| match w {
            Constraint::Percentage(p) => p <= 100,
            _ => true,
        };
        assert!(
            widths.iter().all(between_0_and_100),
            "Percentages should be between 0 and 100 inclusively."
        );
        self.widths = widths;
        self
    }

    /// Change/set rows of the table
    pub(crate) fn rows<R>(mut self, rows: R) -> Self
    where
        R: IntoIterator<Item = Row<'a>>,
    {
        self.rows = rows.into_iter().collect();
        self
    }

    /// Change/set overall table style
    pub(crate) fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Change/set symbol that indicates item is selected
    pub(crate) fn mark_symbol(mut self, mark_symbol: &'a str) -> Self {
        self.mark_symbol = Some(mark_symbol);
        self
    }

    /// Change/set highlight symbol, used to indicate that item is selected in
    /// multi-selection mode
    pub(crate) fn unmark_symbol(mut self, unmark_symbol: &'a str) -> Self {
        self.unmark_symbol = Some(unmark_symbol);
        self
    }

    /// Change/set highlight symbol, used to indicate that item is selected in
    /// multi-selection mode
    pub(crate) fn mark_highlight_symbol(mut self, mark_highlight_symbol: &'a str) -> Self {
        self.mark_highlight_symbol = Some(mark_highlight_symbol);
        self
    }

    /// Change/set highlight symbol, used to indicate that item is not selected
    pub(crate) fn unmark_highlight_symbol(mut self, unmark_highlight_symbol: &'a str) -> Self {
        self.unmark_highlight_symbol = Some(unmark_highlight_symbol);
        self
    }

    /// Change/set highlight of the tags
    pub(crate) fn highlight_tags(mut self, highlight_tags: bool) -> Self {
        self.highlight_tags = highlight_tags;
        self
    }

    /// Change/set highlight symbol, used to indicate that item is selected
    pub(crate) fn highlight_symbol(mut self, highlight_symbol: &'a str) -> Self {
        self.highlight_symbol = Some(highlight_symbol);
        self
    }

    /// Change/set highlight style when item is selected
    pub(crate) fn highlight_style(mut self, highlight_style: Style) -> Self {
        self.highlight_style = highlight_style;
        self
    }

    /// Change/set space between columns of data (filename and tags)
    pub(crate) fn column_spacing(mut self, spacing: u16) -> Self {
        self.column_spacing = spacing;
        self
    }

    /// Change/set size of vertical gap between the header and the data
    pub(crate) fn header_gap(mut self, gap: u16) -> Self {
        self.header_gap = gap;
        self
    }
}

impl<H> StatefulWidget for Table<'_, H>
where
    H: Iterator + fmt::Debug + Sync + Clone,
    H::Item: Display,
{
    type State = TableState;

    fn render(mut self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        buf.set_style(area, self.style);

        // Render block if necessary and get the drawing area
        let table_area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            },
            None => area,
        };

        let mut solver = Solver::new();
        let mut var_indices = HashMap::new();
        let mut ccs = Vec::new();
        let mut variables = Vec::new();
        for i in 0..self.widths.len() {
            let var = cassowary::Variable::new();
            variables.push(var);
            var_indices.insert(var, i);
        }
        for (i, constraint) in self.widths.iter().enumerate() {
            ccs.push(variables[i] | GE(WEAK) | 0.);
            ccs.push(match *constraint {
                Constraint::Length(v) => variables[i] | EQ(MEDIUM) | f64::from(v),
                Constraint::Percentage(v) =>
                    variables[i] | EQ(WEAK) | (f64::from(v * area.width) / 100.0),
                Constraint::Ratio(n, d) =>
                    variables[i] | EQ(WEAK) | (f64::from(area.width) * f64::from(n) / f64::from(d)),
                Constraint::Min(v) => variables[i] | GE(WEAK) | f64::from(v),
                Constraint::Max(v) => variables[i] | LE(WEAK) | f64::from(v),
            });
        }
        solver
            .add_constraint(
                variables
                    .iter()
                    .fold(Expression::from_constant(0.), |acc, v| acc + *v)
                    | LE(REQUIRED)
                    | f64::from(
                        area.width - 2 - (self.column_spacing * (variables.len() as u16 - 1)),
                    ),
            )
            .unwrap();
        solver.add_constraints(&ccs).unwrap();

        let mut solved_widths = vec![0; variables.len()];
        for &(var, value) in solver.fetch_changes() {
            let index = var_indices[&var];
            let value = if value.is_sign_negative() {
                0
            } else {
                value as u16
            };
            solved_widths[index] = value;
        }

        let mut y = table_area.top();
        let mut x = table_area.left();

        // Draw header
        let mut header_index = usize::MAX;
        let mut index = 0;
        if y < table_area.bottom() {
            for (w, t) in solved_widths.iter().zip(self.header.by_ref()) {
                buf.set_stringn(
                    x,
                    y,
                    match self.header_alignment {
                        Alignment::Left =>
                            format!("{symbol:>width$}", symbol = " ", width = *w as usize),
                        Alignment::Center =>
                            format!("{symbol:^width$}", symbol = " ", width = *w as usize),
                        Alignment::Right =>
                            format!("{symbol:<width$}", symbol = " ", width = *w as usize),
                    },
                    *w as usize,
                    self.header_style,
                );
                buf.set_stringn(
                    x,
                    y,
                    match self.header_alignment {
                        Alignment::Left =>
                            format!("{symbol:>width$}", symbol = t, width = *w as usize),
                        Alignment::Center =>
                            format!("{symbol:^width$}", symbol = t, width = *w as usize),
                        Alignment::Right =>
                            format!("{symbol:<width$}", symbol = t, width = *w as usize),
                    },
                    *w as usize,
                    self.header_style,
                );
                // buf.set_stringn(x, y, format!("{}", t), *w as usize, self.header_style);
                // Seems unncessary to have to convert from string
                header_index = index;
                x += *w + self.column_spacing;
                index += 1;
            }
        }
        y += 1 + self.header_gap;

        // Use highlight_style only if something is selected
        let is_selected = state.current_selection().is_some();
        let (selected, highlight_style) = if is_selected {
            (state.current_selection(), self.highlight_style)
        } else {
            (None, self.style)
        };

        // Perhaps increasing the spacing between highlight symbol and the line when
        // using this particular UTF character would be good. There isn't much space
        // •
        let highlight_symbol = match state.mode {
            TableSelection::Multiple => {
                // This format of let s = ... is much easier to read IMO
                let s = self.highlight_symbol.unwrap_or("\u{2022}").trim_end();
                format!("{} ", s)
            },
            TableSelection::Single => self.highlight_symbol.unwrap_or("").to_string(),
        };

        // ✔
        let mark_symbol = match state.mode {
            TableSelection::Multiple => {
                let s = self.mark_symbol.unwrap_or("\u{2714}").trim_end();
                format!("{} ", s)
            },
            TableSelection::Single => self.highlight_symbol.unwrap_or("").to_string(),
        };

        let blank_symbol = match state.mode {
            TableSelection::Multiple => {
                let s = self.unmark_symbol.unwrap_or(" ").trim_end();
                format!("{} ", s)
            },
            TableSelection::Single => " ".repeat(highlight_symbol.width()),
        };

        // ⦿
        let mark_highlight_symbol = {
            let s = self.mark_highlight_symbol.unwrap_or("\u{29bf}").trim_end();
            format!("{} ", s)
        };

        // ⦾
        let unmark_highlight_symbol = {
            let s = self
                .unmark_highlight_symbol
                .unwrap_or("\u{29be}")
                .trim_end();
            format!("{} ", s)
        };

        // Draw rows
        let default_style = Style::default();
        if y < table_area.bottom() {
            let remaining = (table_area.bottom() - y) as usize;

            // Make sure the table shows the selected item
            state.offset = if let Some(s) = selected {
                if s >= remaining + state.offset - 1 {
                    s + 1 - remaining
                } else if s < state.offset {
                    s
                } else {
                    state.offset
                }
            } else {
                0
            };

            // super::dump_and_exit(|| println!("STYLE: {:#?}", self.clone()));
            for (i, row) in self
                .rows
                .into_iter()
                .skip(state.offset)
                .take(remaining)
                .enumerate()
            {
                // let (r, c) = (table_area.top() + y, table_area.left());
                //
                // let table_row_area = Rect {
                //     x:      c,
                //     y:      r,
                //     width:  table_area.width,
                //     height: row.height,
                // };
                //
                // buf.set_style(table_row_area, row.style);

                let symbol = {
                    if Some(i) == state.current_selection().map(|s| s - state.offset) {
                        match state.mode {
                            TableSelection::Multiple => {
                                if state.marked.contains(&(i + state.offset)) {
                                    mark_highlight_symbol.to_string()
                                } else {
                                    unmark_highlight_symbol.to_string()
                                }
                            },
                            TableSelection::Single => highlight_symbol.to_string(),
                        }
                    } else if state.marked.contains(&(i + state.offset)) {
                        mark_symbol.to_string()
                    } else {
                        blank_symbol.to_string()
                    }
                };

                let highlight_tags = self.highlight_tags;
                let select_style = |style: Style| -> Style {
                    if Some(i) == state.current_selection().map(|s| s - state.offset) {
                        highlight_style
                    } else {
                        style
                    }
                };
                let should_highlight_tags = |style: Style| -> Style {
                    if highlight_tags {
                        select_style(style)
                    } else {
                        style
                    }
                };

                x = table_area.left();

                // Cell { content: Text { lines: [ Spans [ Span {} ]],  }, style: Style {} }
                for (oidx, (w, cell)) in solved_widths.iter().zip(row.cells).enumerate() {
                    // Don't think this is usually filled with style
                    buf.set_style(area, cell.style);

                    // Spans { content: "tag name", style: Style { fg: Some(color) } }
                    for (idx, spans) in cell.content.lines.iter().enumerate() {
                        if idx as u16 >= area.height {
                            break;
                        }

                        let mut remaining_width = *w;
                        let mut x = x;
                        let span_length = spans.0.len();

                        for (ii, span) in spans.0.iter().enumerate() {
                            if remaining_width == 0 {
                                break;
                            }

                            let pos = if oidx == 0 {
                                // This sets the filename
                                buf.set_stringn(
                                    x,
                                    y + i as u16,
                                    format!(
                                        "{symbol:^width$}",
                                        symbol = "",
                                        width = area.width as usize
                                    ),
                                    remaining_width as usize,
                                    select_style(span.style),
                                );
                                buf.set_stringn(
                                    x,
                                    y + i as u16,
                                    if oidx == header_index {
                                        #[allow(clippy::match_same_arms)]
                                        let symbol = match state.mode {
                                            TableSelection::Single => &symbol,
                                            TableSelection::Multiple => &symbol,
                                        };
                                        // Unsure when this gets called
                                        format!(
                                            "{symbol}{cont:>width$}",
                                            symbol = symbol,
                                            cont = span.content.as_ref(),
                                            width = (remaining_width as usize)
                                                .saturating_sub(symbol.to_string().width())
                                        )
                                    } else {
                                        format!(
                                            "{symbol}{cont:<width$}",
                                            symbol = symbol,
                                            cont = span.content.as_ref(),
                                            width = (remaining_width as usize)
                                                .saturating_sub(symbol.to_string().width())
                                        )
                                    },
                                    remaining_width as usize,
                                    select_style(span.style),
                                )
                            } else if span_length > 1 && ii < span_length {
                                // If tag length is greater than one and it's not the last
                                // This sets the tags
                                buf.set_stringn(
                                    x,
                                    y + i as u16,
                                    format!("{} ", span.content.as_ref()),
                                    (remaining_width as usize)
                                        .saturating_sub(" ".to_string().width()),
                                    should_highlight_tags(span.style),
                                )
                            } else {
                                // If it's a single tag or the last tag
                                buf.set_stringn(
                                    x,
                                    y + i as u16,
                                    span.content.as_ref(),
                                    remaining_width as usize,
                                    should_highlight_tags(span.style),
                                )
                            };

                            let new_w = pos.0.saturating_sub(x);
                            x = pos.0;
                            remaining_width = remaining_width.saturating_sub(new_w);
                        }
                    }

                    x += *w + self.column_spacing;
                }

                // if state.current_selection().map_or(false, |s| s == i) {
                //     buf.set_style(table_row_area, highlight_style);
                // }
            }
        }
    }
}

impl<H> Widget for Table<'_, H>
where
    H: Iterator + fmt::Debug + Sync + Clone,
    H::Item: Display,
{
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut state = TableState::default();
        StatefulWidget::render(self, area, buf, &mut state);
    }
}
