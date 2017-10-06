//! Iterator tools for `xml-rs`

use xml::reader::XmlEvent;
use xml::reader::Result as XmlResult;

/// Iterator for iterating over all the contents of the tag that we're currently inside
pub struct XmlContents<It: Iterator<Item=XmlResult<XmlEvent>>> {
    it: It,
    depth: u32,
}

impl<It: Iterator<Item=XmlResult<XmlEvent>>> XmlContents<It> {
    /// Creates a new XmlContents iterator from an iterator that is inside of the element
    /// that the new contents iterator is to iterate over the contents of
    pub fn new_inside(it: It) -> XmlContents<It> {
        XmlContents {
            it: it,
            depth: 1
        }
    }

    /// Checks if this XmlContents iterator has terminated (i.e. is outside the scope of the
    /// element that it was reading
    #[inline(always)]
    fn is_outside_scope(&self) -> bool {
        self.depth < 1
    }
}

impl<It: Iterator<Item=XmlResult<XmlEvent>>> Iterator for XmlContents<It> {
    type Item = XmlResult<XmlEvent>;
    fn next(&mut self) -> Option<XmlResult<XmlEvent>> {
        if self.is_outside_scope() {
            None
        } else {
            let ret = self.it.next().map(|r| match r {
                e @ Ok(XmlEvent::StartElement { .. }) => {
                    self.depth = self.depth + 1;
                    e
                },
                e @ Ok(XmlEvent::EndElement { .. }) => {
                    self.depth = self.depth - 1;
                    e
                },
                e => e,
            });
            if self.is_outside_scope() {
                None
            } else {
                ret
            }
        }
    }
}

pub trait XmlIteratorExtensions: Iterator<Item=XmlResult<XmlEvent>> where Self: Sized {
    fn xml_contents(self) -> XmlContents<Self>;
}

impl<It: Iterator<Item=XmlResult<XmlEvent>>> XmlIteratorExtensions for It {
    fn xml_contents(self) -> XmlContents<Self> {
        XmlContents::new_inside(self)
    }
}
