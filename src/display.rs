use std::borrow::Cow;

use anstyle_parse::state::{state_change, Action, State};

pub(crate) fn clean(input: &str) -> Cow<'_, str> {
    if !input
        .chars()
        .any(|ch| ch.is_control() && ch != '\n' && ch != '\t')
    {
        return Cow::Borrowed(input);
    }

    let mut output = String::with_capacity(input.len());
    let mut state = State::Ground;
    for ch in input.chars() {
        if !ch.is_ascii() {
            if !is_control_string(state) {
                // The input is already valid UTF-8. A non-ASCII character cannot
                // finish an ESC/CSI sequence; recover without losing visible text.
                state = State::Ground;
                if !ch.is_control() {
                    output.push(ch);
                }
            }
            continue;
        }
        let (next, action) = state_change(state, ch as u8);
        if next != State::Anywhere {
            state = next;
        }
        if (action == Action::Print && !ch.is_control())
            || (action == Action::Execute && (ch == '\n' || ch == '\t'))
        {
            output.push(ch);
        }
    }
    Cow::Owned(output)
}

fn is_control_string(state: State) -> bool {
    matches!(
        state,
        State::OscString
            | State::SosPmApcString
            | State::DcsEntry
            | State::DcsIgnore
            | State::DcsIntermediate
            | State::DcsParam
            | State::DcsPassthrough
    )
}
