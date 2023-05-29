use std::io::Cursor;

use derive_builder::Builder;
use ical::generator::IcalCalendar;
use reqwest::Method;
use xml::reader::XmlEvent;
use xml::EventReader;

use crate::api::ApiClient;

use super::calendar::Calendar;
use super::calendar::CalendarComponents;
use super::calendar::{CALDAV_NS, DAV_NS};

extern crate ical;

#[allow(dead_code)]
#[derive(Default, Debug, Clone, Builder)]
pub struct Todo {
    href: String,
    calendar: IcalCalendar,
}

impl Todo {
    pub fn get_todo(self) -> String {
        // self.calendar.todos
        todo!()
    }
}

pub async fn get_todos(api: &ApiClient, calendar: &Calendar) -> Option<Vec<Todo>> {
    if calendar.has_component(CalendarComponents::Todo) {
        println!("CAN TODO");
        let body =
            "<x1:calendar-query xmlns:x0=\"DAV:\" xmlns:x1=\"urn:ietf:params:xml:ns:caldav\">
            <x0:prop>
              <x0:getcontenttype/><x0:getetag/><x1:calendar-data/>
            </x0:prop>
            <x1:filter>
              <x1:comp-filter name=\"VCALENDAR\">
                <x1:comp-filter name=\"VTODO\">
<!-- Filter Option for only completed or not completed -->
                </x1:comp-filter>
              </x1:comp-filter>
            </x1:filter>
          </x1:calendar-query>";

        let url = api.build_url(calendar.get_url());
        let method = Method::from_bytes(b"REPORT").unwrap();
        let request = api
            .get_client()
            .request(method, url)
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/xml; charset=utf-8",
            )
            .header("Depth", "1")
            .body(body)
            .send();

        // we should check for HTTP 207 (Multi Response)
        let response = request.await.unwrap();
        let responsexml = response.text().await.unwrap();

        let reader = Cursor::new(responsexml);
        let parser = EventReader::new(reader);

        let mut builder = TodoBuilder::default();
        let mut todos: Vec<Todo> = Vec::new();
        let mut field = "";

        // rebuild this in a better way - see https://github.com/Enet4/dicom-rs/blob/b13c13facf5ddcedce92344f86c334109f958aa6/dictionary_builder/main.rs#L141
        for e in parser {
            match e {
                Ok(XmlEvent::StartElement { name, .. }) => {
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
                    if elementns.eq(CALDAV_NS) && name.local_name == "calendar-data" {
                        field = "calendar-data";
                    }
                }
                Ok(XmlEvent::EndElement { name }) => {
                    let elementns = if name.namespace.is_some() {
                        name.namespace.unwrap()
                    } else {
                        String::new()
                    };

                    if elementns == DAV_NS && name.local_name == "response" {
                        let option_todo = builder.build();
                        if let Ok(todo) = option_todo {
                            todos.push(todo);
                        }

                        builder = TodoBuilder::default().to_owned();
                    }
                }
                Ok(XmlEvent::Characters(data)) => {
                    let value = data.trim().replace("\u{200b}", "");
                    match field {
                        "href" => {
                            builder = builder.href(value).to_owned();
                        }
                        "calendar-data" => {
                            let buf = Cursor::new(value);
                            let mut reader = ical::IcalParser::new(buf);

                            if let Some(Ok(icalendar)) = reader.next() {
                                builder.calendar(icalendar);
                            }
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

        if !todos.is_empty() {
            return Some(todos);
        }
    }

    None
}
