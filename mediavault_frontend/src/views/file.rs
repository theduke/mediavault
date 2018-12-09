use js_sys::Date;
use mediavault_common::types as t;

type Error = String;

#[derive(Clone, Debug)]
pub enum ContainerMessage {
    Load { hash: String },
    Result(Result<t::File, Error>),
    File(Message),
}

#[derive(Default, Clone, Debug)]
pub struct FileContainer {
    pub result: Option<Result<FileView, Error>>,
}

impl draco::App for FileContainer {
    type Message = ContainerMessage;

    fn update(&mut self, mailbox: &draco::Mailbox<Self::Message>, message: Self::Message) {
        match message {
            ContainerMessage::Load { hash } => {
                mailbox.spawn(crate::api::file(&hash), |res| {
                    ContainerMessage::Result(res)
                });
            }
            ContainerMessage::Result(res) => {
                self.result = Some(res.map(FileView::new));
            }
            ContainerMessage::File(msg) => {
                match self.result.as_mut() {
                    None | Some(Err(_)) => {
                        error!("Invalid file event received: no file loaded");
                    }
                    Some(Ok(ref mut view)) => {
                        view.update(&mailbox.clone().map(ContainerMessage::File), msg);
                    }
                }
            }
        }
    }

    fn render(&self) -> draco::Node<Self::Message> {
        use draco::html as h;
        match self.result {
            None => {
                h::div().push("Loading").into()
            }
            Some(Err(ref e)) => {
                h::div().push(format!("Error: {}", e)).into()
            }
            Some(Ok(ref view)) => {
                view.render().map(ContainerMessage::File).into()
            }
        }
    }
}


#[derive(Debug, Clone)]
pub enum Edit {
    Title(String),
    Description(String),
    TagRemove(String),
    TagAdd(String),
}

#[derive(Debug, Clone)]
pub enum Message {
    Show(t::File),
    Edit(Edit),
    Save,
    Saved(t::File),
    Error(String),
}

#[derive(Debug, Clone)]
pub struct FileView {
    pub file: t::File,

    // Edit fields.
    title: Option<String>,
    description: Option<String>,
    tag_input: String,
    tags: Option<Vec<String>>,
    last_edit: Option<f64>,

    // Saving state.
    saving: bool,
    error: Option<String>,
}

impl FileView {
    pub fn new(file: t::File) -> Self {
        Self {
            file,
            title: None,
            description: None,
            tag_input: String::new(),
            tags: None,
            last_edit: None,
            saving: false,
            error: None,
        }
    }

    fn title(&self) -> String {
        self.title
            .as_ref()
            .or(self.file.meta.title.as_ref())
            .map(|s| s.clone())
            .unwrap_or(String::new())
    }

    fn description(&self) -> String {
        self.description
            .as_ref()
            .or(self.file.meta.description.as_ref())
            .map(|s| s.clone())
            .unwrap_or(String::new())
    }

    fn tags(&self) -> &Vec<String> {
        self.tags.as_ref().unwrap_or(&self.file.meta.tags)
    }
}

impl draco::App for FileView {
    type Message = Message;

    fn update(&mut self, mailbox: &draco::Mailbox<Self::Message>, message: Self::Message) {
        use self::Message::*;
        match message {
            Show(f) => self.file = f,
            Edit(edit) => {
                match edit {
                    self::Edit::Title(title) => {
                        self.title = Some(title);
                    }
                    self::Edit::Description(description) => {
                        self.description = Some(description);
                    }
                    self::Edit::TagRemove(tag) => {
                        let mut tags = self.tags().clone();
                        tags.retain(|t| t != &tag);
                        self.tags = Some(tags);
                    }
                    self::Edit::TagAdd(tag) => {
                        // First, check if tag is ready to be added.
                        if tag.ends_with(' ') || tag.ends_with(',') {
                            let new_tag = tag[0..tag.len() - 2].to_string();

                            // TODO: check tag validity with regex.
                            if new_tag.len() > 0 {
                                let mut tags = self.tags().clone();
                                if !tags.contains(&tag) {
                                    tags.push(tag);
                                }
                                self.tags = Some(tags);
                                self.tag_input = String::new();
                            }
                        } else {
                            self.tag_input = tag.trim().to_string();
                            // NOTE: early return in case of non-complete tag.
                            return;
                        }
                    }
                }
                self.last_edit = Some(Date::now());
                mailbox.send_after(5000, || Message::Save);
            }
            Save => {
                if let Some(last_edit) = self.last_edit {
                    let time_passed = Date::now() - last_edit;
                    let should_save = time_passed > 5000.0;
                    if should_save {
                        self.saving = true;
                        mailbox.spawn(
                            crate::api::file_update(&t::FileUpdate {
                                hash: self.file.info.hash.clone(),
                                title: self.title.clone(),
                                description: self.description.clone(),
                                tags: self.tags.clone(),
                            }),
                            |res| match res {
                                Ok(d) => Message::Saved(d),
                                Err(e) => {
                                    log!("fetch error: {}", e);
                                    Message::Error(e)
                                }
                            },
                        );
                    }
                }
            }
            Saved(f) => {
                self.file = f;
                self.title = None;
                self.description = None;
                self.last_edit = None;
                self.saving = false;
            }
            Error(e) => {
                self.error = Some(e);
            }
        }
    }

    fn render(&self) -> draco::Node<Self::Message> {
        use draco::html as h;

        let title_input = h::input()
            .attr("placeholder", "Title...")
            .attr("value", self.title())
            .on_input(|value| Message::Edit(Edit::Title(value)));
        let title_input = if self.saving {
            title_input.attr("disabled", "disabled")
        } else {
            title_input
        };
        let title = h::div().push(title_input);

        let description_textarea = h::textarea()
            .attr("placeholder", "Description...")
            .push(self.description())
            .on_input(|value| Message::Edit(Edit::Description(value)));
        let description_textarea = if self.saving {
            description_textarea.attr("disabled", "disabled")
        } else {
            description_textarea
        };
        let description = h::div().push(description_textarea);

        let tags = h::div()
            .class("m-TagEditor-Tags")
            .append(self.tags().iter().map(|tag| {
                let tag_clone = tag.clone();
                h::div()
                    .class("m-TagEditor-Tag")
                    .push(h::div().push(tag))
                    .push(
                        h::div()
                            .class("m-TagEditor-Remove")
                            .push("x")
                            .on("click", move |_| {
                                Message::Edit(Edit::TagRemove(tag_clone.clone()))
                            }),
                    )
            }));
        let tag_input = h::input()
            .attr("value", self.tag_input.clone())
            .attr("placeholder", "Add tags...")
            .on_input(|value| Message::Edit(Edit::TagAdd(value)));

        let tag_input = if self.saving {
            tag_input.attr("disabled", "disabled")
        } else {
            tag_input
        };

        let tag_editor = h::div()
            .class("m-TagEditor")
            .push(tags)
            .push(h::div().push(tag_input));

        let sidebar = h::div()
            .class("m-FileView-SideBar")
            .push(title)
            .push(description)
            .push(tag_editor);

        let viewer = h::div()
            .class("m-FileView-Viewer")
            .push(h::img().attr("src", format!("/media/{}", self.file.path)));

        h::div()
            .class("m-FileView")
            .push(sidebar)
            .push(viewer)
            //.push(h::button().push("Reset").on("click", |_| Message::Reset))
            .into()
    }
}
