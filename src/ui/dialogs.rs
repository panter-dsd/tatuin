// SPDX-License-Identifier: MIT

mod add_edit_task;
mod dialog;
mod key_bindings_help;
mod list;
mod states;
mod text_input;

pub use add_edit_task::Dialog as AddEditTaskDialog;
pub use dialog::DialogTrait;
pub use key_bindings_help::Dialog as KeyBindingsHelpDialog;
pub use list::Dialog as ListDialog;
pub use states::Dialog as StatesDialog;
pub use text_input::Dialog as TextInputDialog;
