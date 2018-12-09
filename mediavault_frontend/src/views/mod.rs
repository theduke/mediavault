pub mod file;
pub mod files;

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Route {
    Home,
    NotFound,
    File {
        hash: String,
    },
}

impl Route {
    fn from_url(url: &draco::router::Url) -> Route {
        use draco::router::{parse, param};
        parse(url)
            .alt((), |()| Route::Home)
            .alt(("file", param()), |((), hash)| Route::File{
                hash,
            })
            .value()
            .unwrap_or(Route::NotFound)
    }

    fn to_path(&self) -> String {
        use self::Route::*;
        match self {
            Home => "/".to_string(),
            NotFound => "/not-found".to_string(),
            File{ hash } => format!("/file/{}", hash),
        }
    }

    pub fn goto(route: &Self) {
        draco::router::push(draco::router::Mode::History, &route.to_path());
    }
}

#[derive(Debug, Clone)]
pub enum View {
    Files(files::Files),
    File(file::FileContainer),
}

#[derive(Debug)]
pub enum Message {
    Start,
    UrlChange(draco::router::Url),

    Show(View),

    Files(files::Message),
    File(file::ContainerMessage),
}

#[derive(Debug)]
pub struct Root {
    view: View,
    // Needed to keep the history subscription alive.
    history_subscription: Option<draco::Unsubscribe>,
    current_route: Route,

    // Caches.
    cache_files: Option<files::Files>,
}

impl Default for Root {
    fn default() -> Self {
        Root {
            view: View::Files(files::Files::default()),
            history_subscription: None,
            current_route: Route::Home,
            cache_files: None,
        }
    }
}

impl draco::App for Root {
    type Message = Message;

    fn update(&mut self, mailbox: &draco::Mailbox<Self::Message>, message: Self::Message) {
        use self::Message::*;

        log!("MSG: {:#?}", message);

        match message {
            Start => {
              self.history_subscription = Some(mailbox.subscribe(
                    draco::router::Router::new(draco::router::Mode::History),
                    Message::UrlChange,
              ));
            },
            UrlChange(url) => {
                let route = Route::from_url(&url);
                if self.current_route != route {
                    log!("Url changed: {:#?}", url);
                    self.current_route = route.clone();

                    let view = match &route {
                        &Route::Home | &Route::NotFound => {
                            View::Files(self.cache_files.take().unwrap_or(files::Files::default()))
                        },
                        &Route::File { ref hash } => {
                            let msg = Message::File(file::ContainerMessage::Load { hash: hash.to_string() });
                            mailbox.send(msg);
                            View::File(file::FileContainer::default())
                        },
                    };
                    self.update(mailbox, Message::Show(view));
                }
            },
            Show(v) => {
                let old_view = ::std::mem::replace(&mut self.view, v);
                match old_view {
                    View::Files(f) => {
                        // TODO: don't cache if loading.
                        self.cache_files = Some(f);
                    },
                    _ => {},
                }
            },
            Files(msg) => match &mut self.view {
                View::Files(ref mut v) => {
                    v.update(&mailbox.clone().map(|m| Message::Files(m)), msg);
                }
                _ => {}
            },
            File(msg) => match &mut self.view {
                View::File(ref mut v) => {
                    v.update(&mailbox.clone().map(|m| Message::File(m)), msg);
                }
                _ => {}
            },
        }
    }

    fn render(&self) -> draco::Node<Self::Message> {
        use draco::html as h;

        use self::View::*;
        let view = match &self.view {
            Files(v) => v.render().map(Message::Files),
            File(v) => v.render().map(Message::File),
        };

        h::div()
            .class("m-Root")
            .push(view)
            //.push(h::button().push("Reset").on("click", |_| Message::Reset))
            .into()
    }
}
