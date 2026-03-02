pub mod create_folder;
pub mod move_picker;
pub mod rename;
pub mod upload;

use leptos::prelude::*;

use crate::app::ModalState;

/// Renders whichever modal is currently active (or nothing).
#[component]
pub fn Modals() -> impl IntoView {
    let modal = use_context::<RwSignal<Option<ModalState>>>().expect("modal context missing");

    move || match modal.get() {
        None => view! { <div></div> }.into_any(),
        Some(ModalState::CreateFolder) => view! { <create_folder::CreateFolderModal /> }.into_any(),
        Some(ModalState::Rename {
            path,
            current_name,
            kind,
        }) => view! {
            <rename::RenameModal path=path current_name=current_name kind=kind />
        }
        .into_any(),
        Some(ModalState::Move { items }) => {
            view! { <move_picker::MovePickerModal items=items /> }.into_any()
        }
        Some(ModalState::Upload { folder_path }) => {
            view! { <upload::UploadModal folder_path=folder_path /> }.into_any()
        }
    }
}
