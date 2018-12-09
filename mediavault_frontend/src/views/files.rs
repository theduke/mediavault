use draco::{element::NonKeyedElement as Elem, html as h};
use mediavault_common::types as t;

#[derive(Debug, Clone)]
pub enum Message {
    Query(t::FileQuery),
    Data(t::FilesPage),
    Error(String),
    ShowFile(t::File),
}

#[derive(Debug, Clone)]
pub struct Files {
    query: t::FileQuery,
    data: Option<t::FilesPage>,
    error: Option<String>,
}

impl Default for Files {
    fn default() -> Self {
        Files {
            query: t::FileQuery::default(),
            data: None,
            error: None,
        }
    }
}

fn view_pager(f: &Files) -> Elem<Message> {
    let mut p = h::div().class("m-Files-pager");

    if let Some(data) = f.data.as_ref() {
        let page = data.page;

        if page > 1 {
            let mut q = f.query.clone();
            q.page -= 1;
            p = p.push(
                h::div()
                    .class("m-Files-pager-prev")
                    .on("click", move |_| Message::Query(q.clone()))
                    .push("Prev"),
            );
        }

        if data.has_more() {
            let mut q = f.query.clone();
            q.page += 1;

            p = p.push(
                h::div()
                    .class("m-Files-pager-next")
                    .push("Next")
                    .on("click", move |_| Message::Query(q.clone())),
            );
        }
    }

    p
}

fn view_filter(q: &t::FileQuery) -> Elem<Message> {
    let tags = h::div().push(h::label().push("Tags")).push(
        h::input()
            .attr("type", "text")
            .attr("placeholder", "Tags..."),
    );

    h::div().class("m-Files-Filter").push(tags)
}

fn view_files(p: Option<&t::FilesPage>) -> Elem<Message> {
    match p {
        Some(p) => h::div()
            .class("m-Files-Viewer")
            .append(p.items.iter().map(|f| {

                let content = match f.info.kind {
                    t::FileKind::Image => {
                        h::img()
                            .class("m-Files-Image")
                            .attr("src", format!("/media/{}", f.path))
                    }
                    t::FileKind::Video => {
                        h::span().push(&f.path)
                    }
                    t::FileKind::Audio => {
                        h::span().push(&f.path)
                    }
                    t::FileKind::Other => {
                        h::span().push(&f.path)
                    }
                };

                let file_clone = f.clone();
                h::div()
                    .class("m-Files-File")
                    .push(content)
                    .on("click", move |_| Message::ShowFile(file_clone.clone()))
            })),
        None => h::div().push("loading"),
    }
}

impl draco::App for Files {
    type Message = Message;

    fn update(&mut self, mailbox: &draco::Mailbox<Self::Message>, message: Self::Message) {
        use self::Message::*;
        match message {
            Query(q) => {
                self.query = q.clone();

                mailbox.spawn(crate::api::files(q), |res| match res {
                    Ok(d) => Message::Data(d),
                    Err(e) => {
                        log!("fetch error: {}", e);
                        Message::Error(e)
                    }
                });
            }
            Data(data) => {
                self.data = Some(data);
            }
            Error(e) => {
                // TODO: show error msg.
                self.error = Some(e);
            }
            ShowFile(f) => {
                super::Route::goto(&super::Route::File { hash: f.info.hash });
            }
        }
    }

    fn render(&self) -> draco::Node<Self::Message> {
        h::div()
            .class("m-Files")
            .push(view_filter(&self.query))
            .push(
                h::div()
                    .class("m-Files-Browser")
                    .push(view_files(self.data.as_ref()))
                    .push(view_pager(self)),
            )
            //.push(h::button().push("Reset").on("click", |_| Message::Reset))
            .into()
    }
}
