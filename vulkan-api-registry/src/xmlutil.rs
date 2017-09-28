use xml::reader::XmlEvent;
use xml::reader::Result as XmlResult;

pub struct Contents<'a, It: Iterator<Item=XmlResult<XmlEvent>> + 'a> {
    depth: u32,
    events: &'a mut It,
}

impl<'a, It: Iterator<Item=XmlResult<XmlEvent>>> Contents<'a, It> {
    pub fn new(events: &'a mut It) -> Contents<'a, It> {
        Contents {
            depth: 1,
            events: events
        }
    }
}

impl<'a, It: Iterator<Item=XmlResult<XmlEvent>>> Iterator for Contents<'a, It> {
    type Item = XmlResult<XmlEvent>;
    fn next(&mut self) -> Option<XmlResult<XmlEvent>> {
        if self.depth <= 0 {
            None
        } else {
            self.events.next().and_then(|e| match e {
                evt @ Ok(XmlEvent::StartElement { .. }) => {
                    self.depth = self.depth + 1;
                    Some(evt)
                },
                evt @ Ok(XmlEvent::EndElement { .. }) => {
                    self.depth = self.depth - 1;
                    if self.depth <= 0 {
                        None
                    } else {
                        Some(evt)
                    }
                },
                result => Some(result),
            })
        }
    }
}
