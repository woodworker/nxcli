use derive_builder::Builder;
use reqwest::Method;
use std::{fmt, io::Cursor, str::FromStr};
use xml::reader::{EventReader, XmlEvent};

use crate::api::ApiClient;

pub const CALDAV_NS: &str = "urn:ietf:params:xml:ns:caldav";
pub const DAV_NS: &str = "DAV:";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CalendarComponents {
    Event = 0,
    Todo = 1,
}

impl FromStr for CalendarComponents {
    type Err = ();

    fn from_str(input: &str) -> Result<CalendarComponents, Self::Err> {
        match input {
            "VTODO" => Ok(CalendarComponents::Todo),
            "VEVENT" => Ok(CalendarComponents::Event),
            _ => Err(()),
        }
    }
}

#[allow(dead_code)]
#[derive(Default, Debug, Clone, Builder)]
pub struct Calendar {
    href: String,
    displayname: String,
    supported_components: Vec<CalendarComponents>,
}

#[allow(dead_code)]
impl Calendar {
    pub fn get_url(&self) -> String {
        self.href.clone()
    }

    pub fn get_displayname(&self) -> String {
        self.displayname.clone()
    }

    pub fn get_supported_components(&self) -> Vec<CalendarComponents> {
        self.supported_components.clone()
    }

    pub fn has_component(&self, component: CalendarComponents) -> bool {
        self.get_supported_components()
            .iter()
            .any(|&item| item == component)
    }

    // pub fn
}

impl fmt::Display for Calendar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.displayname, self.href)
    }
}

// #[derive(Default, Debug, Clone)]
// struct CalenderBuilder {
//     href: Option<String>,
//     displayname: Option<String>,
//     supported_components: Vec<CalendarComponents>,
// }
// impl CalenderBuilder {
//     pub fn new() -> Self {
//         Self::default()
//     }

//     pub fn build(self) -> Option<Calendar> {
//         if self.href.is_none() || self.displayname.is_none() || self.supported_components.is_empty()
//         {
//             return None;
//         }

//         Some(Calendar {
//             href: self.href.unwrap(),
//             displayname: self.displayname.unwrap(),
//             supported_components: self.supported_components,
//         })
//     }

//     pub fn set_href(mut self, href: impl Into<String>) -> Self {
//         self.href = Some(href.into());
//         self
//     }

//     pub fn set_displayname(mut self, displayname: impl Into<String>) -> Self {
//         self.displayname = Some(displayname.into());
//         self
//     }

//     pub fn add_supported_component(mut self, supported_component: impl Into<String>) -> Self {
//         self.supported_components
//             .push(CalendarComponents::from_str(&supported_component.into()).unwrap());
//         self
//     }
// }

pub async fn get_calendar_list(api: &ApiClient) -> Vec<Calendar> {
    let url = api.build_url("/remote.php/dav/calendars/{user}/");
    let method = Method::from_bytes(b"PROPFIND").unwrap();
    let request = api
        .get_client()
        .request(method, url)
        .header(
            reqwest::header::CONTENT_TYPE,
            "application/xml; charset=utf-8",
        )
        .header("Depth", "1")
        .send();

    let response = request.await.unwrap();
    let responsexml = response.text().await.unwrap();

    let reader = Cursor::new(responsexml);
    let parser = EventReader::new(reader);

    let mut calenders: Vec<Calendar> = Vec::new();
    let mut builder: CalendarBuilder = CalendarBuilder::default().to_owned();
    let mut field = "";
    let mut in_component_set = false;

    // rebuild this in a better way - see https://github.com/Enet4/dicom-rs/blob/b13c13facf5ddcedce92344f86c334109f958aa6/dictionary_builder/main.rs#L141
    for e in parser {
        match e {
            Ok(XmlEvent::StartElement {
                name, attributes, ..
            }) => {
                let elementns = if name.namespace.is_some() {
                    name.namespace.unwrap()
                } else {
                    String::new()
                };

                // href
                if elementns.eq(DAV_NS) && name.local_name == "href" {
                    field = "href";
                }

                // displayname
                if elementns.eq(DAV_NS) && name.local_name == "displayname" {
                    field = "displayname";
                }

                if elementns.eq(CALDAV_NS) && name.local_name == "supported-calendar-component-set"
                {
                    in_component_set = true;
                }

                if elementns.eq(CALDAV_NS) && name.local_name == "comp" && in_component_set {
                    for attr in attributes {
                        if attr.name.local_name == "name" {
                            let mut supported_components: Vec<CalendarComponents> = builder
                                .supported_components
                                .to_owned()
                                .unwrap_or(Vec::new());

                            let component = CalendarComponents::from_str(&attr.value);

                            if let Ok(component_enum) = component {
                                supported_components.push(component_enum);
                                builder = builder
                                    .supported_components(supported_components)
                                    .to_owned();
                            }
                        }
                    }
                }
            }
            Ok(XmlEvent::EndElement { name }) => {
                let elementns = if name.namespace.is_some() {
                    name.namespace.unwrap()
                } else {
                    String::new()
                };

                if elementns == CALDAV_NS && name.local_name == "supported-calendar-component-set" {
                    in_component_set = false;
                }

                if elementns == DAV_NS && name.local_name == "response" {
                    let cal = builder.build();
                    if let Ok(calender) = cal {
                        calenders.push(calender);
                    }

                    builder = CalendarBuilder::default().to_owned();
                }
            }
            Ok(XmlEvent::Characters(data)) => {
                let value = data.trim().replace("\u{200b}", "");
                match field {
                    "href" => {
                        builder = builder.href(value).to_owned();
                    }
                    "displayname" => {
                        builder = builder.displayname(value).to_owned();
                    }
                    _ => {}
                }
                field = "";
            }
            Err(e) => {
                eprintln!("Error: {e}");
                break;
            }
            // There's more: https://docs.rs/xml-rs/latest/xml/reader/enum.XmlEvent.html
            _ => {}
        }
    }

    calenders
}
