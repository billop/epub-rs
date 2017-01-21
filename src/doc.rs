//! Manages the epub doc.
//!
//! Provides easy methods to navigate througth the epub content, cover,
//! chapters, etc.

extern crate xml;
extern crate regex;

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use archive::EpubArchive;

use xmlutils;

#[derive(Debug)]
pub struct DocError { pub error: String }

impl Error for DocError {
    fn description(&self) -> &str { &self.error }
}

impl fmt::Display for DocError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "DocError: {}", self.error)
    }
}

/// Struct to control the epub document
pub struct EpubDoc {
    /// the zip archive
    archive: EpubArchive,

    /// The current chapter, is an spine index
    current: usize,

    /// epub spine ids
    pub spine: Vec<String>,

    /// resource id -> name
    pub resources: HashMap<String, (String, String)>,

    /// The epub metadata stored as key -> value
    ///
    /// #Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # let doc = EpubDoc::new("test.epub");
    /// # let doc = doc.unwrap();
    /// let title = doc.metadata.get("title");
    /// assert_eq!(title.unwrap(), "Todo es mío");
    /// ```
    pub metadata: HashMap<String, String>,

    /// root file base path
    pub root_base: String,

    /// root file full path
    pub root_file: String,
}

impl EpubDoc {
    /// Opens the epub file in `path`.
    ///
    /// Initialize some internal variables to be able to access to the epub
    /// spine definition and to navigate trhough the epub.
    ///
    /// # Examples
    ///
    /// ```
    /// use epub::doc::EpubDoc;
    ///
    /// let doc = EpubDoc::new("test.epub");
    /// assert!(doc.is_ok());
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the epub is broken or if the file doesn't
    /// exists.
    pub fn new(path: &str) -> Result<EpubDoc, Box<Error>> {
        let mut archive = try!(EpubArchive::new(path));
        let spine: Vec<String> = vec!();
        let resources: HashMap<String, (String, String)> = HashMap::new();

        let container = try!(archive.get_container_file());
        let root_file = try!(get_root_file(container));

        // getting the rootfile base directory
        let re = regex::Regex::new(r"/").unwrap();
        let iter: Vec<&str> = re.split(&root_file).collect();
        let count = iter.len();
        let base_path = if count >= 2 { iter[count - 2] } else { "" };

        let mut doc = EpubDoc {
            archive: archive,
            spine: spine,
            resources: resources,
            metadata: HashMap::new(),
            root_file: root_file.clone(),
            root_base: String::from(base_path) + "/",
            current: 0,
        };

        try!(doc.fill_resources());

        Ok(doc)
    }

    /// Returns the id of the epub cover.
    ///
    /// The cover is searched in the doc metadata, by the tag <meta name="cover" value"..">
    ///
    /// # Examples
    ///
    /// ```rust
    /// use epub::doc::EpubDoc;
    ///
    /// let doc = EpubDoc::new("test.epub");
    /// assert!(doc.is_ok());
    /// let mut doc = doc.unwrap();
    ///
    /// let cover_id = doc.get_cover_id().unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the cover path can't be found.
    pub fn get_cover_id(&self) -> Result<String, Box<Error>> {
        match self.metadata.get("cover") {
            Some(id) => Ok(id.to_string()),
            None => Err(Box::new(DocError { error: String::from("Cover not found") }))
        }
    }

    /// Returns the cover as Vec<u8>
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use std::fs;
    /// use std::io::Write;
    /// use epub::doc::EpubDoc;
    ///
    /// let doc = EpubDoc::new("test.epub");
    /// assert!(doc.is_ok());
    /// let mut doc = doc.unwrap();
    ///
    /// let cover_data = doc.get_cover().unwrap();
    ///
    /// let f = fs::File::create("/tmp/cover.png");
    /// assert!(f.is_ok());
    /// let mut f = f.unwrap();
    /// let resp = f.write_all(&cover_data);
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the cover can't be found.
    pub fn get_cover(&mut self) -> Result<Vec<u8>, Box<Error>> {
        let cover_id = try!(self.get_cover_id());
        let cover_data = try!(self.get_resource(&cover_id));
        Ok(cover_data)
    }

    /// Returns the resource content by full path in the epub archive
    ///
    /// # Errors
    ///
    /// Returns an error if the path doesn't exists in the epub
    pub fn get_resource_by_path(&mut self, path: &str) -> Result<Vec<u8>, Box<Error>> {
        let content = try!(self.archive.get_entry(path));
        Ok(content)
    }

    /// Returns the resource content by the id defined in the spine
    ///
    /// # Errors
    ///
    /// Returns an error if the id doesn't exists in the epub
    pub fn get_resource(&mut self, id: &str) -> Result<Vec<u8>, Box<Error>> {
        let path: String = match self.resources.get(id) {
            Some(s) => s.0.to_string(),
            None => return Err(Box::new(DocError { error: String::from("id not found") }))
        };
        let content = try!(self.get_resource_by_path(&path));
        Ok(content)
    }

    /// Returns the resource content by full path in the epub archive, as String
    ///
    /// # Errors
    ///
    /// Returns an error if the path doesn't exists in the epub
    pub fn get_resource_str_by_path(&mut self, path: &str) -> Result<String, Box<Error>> {
        let content = try!(self.archive.get_entry_as_str(path));
        Ok(content)
    }

    /// Returns the resource content by the id defined in the spine, as String
    ///
    /// # Errors
    ///
    /// Returns an error if the id doesn't exists in the epub
    pub fn get_resource_str(&mut self, id: &str) -> Result<String, Box<Error>> {
        let path: String = match self.resources.get(id) {
            Some(s) => s.0.to_string(),
            None => return Err(Box::new(DocError { error: String::from("id not found") }))
        };
        let content = try!(self.get_resource_str_by_path(&path));
        Ok(content)
    }

    /// Returns the resource mime-type
    ///
    /// # Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # let doc = EpubDoc::new("test.epub");
    /// # let doc = doc.unwrap();
    /// let mime = doc.get_resource_mime("portada.png");
    /// assert_eq!("image/png", mime.unwrap());
    /// ```
    /// # Errors
    ///
    /// Fails if the resource can't be found.
    pub fn get_resource_mime(&self, id: &str) -> Result<String, Box<Error>> {
        match self.resources.get(id) {
            Some(&(_, ref res)) => return Ok(res.to_string()),
            None => {}
        }
        Err(Box::new(DocError { error: String::from("id not found") }))
    }

    /// Returns the resource mime searching by source full path
    ///
    /// # Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # let doc = EpubDoc::new("test.epub");
    /// # let doc = doc.unwrap();
    /// let mime = doc.get_resource_mime_by_path("OEBPS/Images/portada.png");
    /// assert_eq!("image/png", mime.unwrap());
    /// ```
    ///
    /// # Errors
    ///
    /// Fails if the resource can't be found.
    pub fn get_resource_mime_by_path(&self, path: &str) -> Result<String, Box<Error>> {
        for (_, v) in self.resources.iter() {
            if v.0 == path {
                return Ok(v.1.to_string());
            }
        }
        Err(Box::new(DocError { error: String::from("path not found") }))
    }

    /// Returns the current chapter content
    ///
    /// The current follows the epub spine order. You can modify the current
    /// calling to `go_next`, `go_prev` or `set_current` methods.
    ///
    /// # Errors
    ///
    /// This call shouldn't fail, but can return an error if the epub doc is
    /// broken.
    pub fn get_current(&mut self) -> Result<Vec<u8>, Box<Error>> {
        let current_id = try!(self.get_current_id());
        self.get_resource(&current_id)
    }

    pub fn get_current_str(&mut self) -> Result<String, Box<Error>> {
        let current_id = try!(self.get_current_id());
        self.get_resource_str(&current_id)
    }

    /// Returns the current chapter mimetype
    ///
    /// # Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # let doc = EpubDoc::new("test.epub");
    /// # let doc = doc.unwrap();
    /// let m = doc.get_current_mime();
    /// assert_eq!("application/xhtml+xml", m.unwrap());
    /// ```
    pub fn get_current_mime(&self) -> Result<String, Box<Error>> {
        let current_id = try!(self.get_current_id());
        self.get_resource_mime(&current_id)
    }

    /// Returns the current chapter full path
    ///
    /// # Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # let doc = EpubDoc::new("test.epub");
    /// # let doc = doc.unwrap();
    /// let p = doc.get_current_path();
    /// assert_eq!("OEBPS/Text/titlepage.xhtml", p.unwrap());
    /// ```
    pub fn get_current_path(&self) -> Result<String, Box<Error>> {
        let current_id = try!(self.get_current_id());
        match self.resources.get(&current_id) {
            Some(&(ref p, _)) => return Ok(p.to_string()),
            None => return Err(Box::new(DocError { error: String::from("Current not found") }))
        }
    }

    /// Returns the current chapter id
    ///
    /// # Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # let doc = EpubDoc::new("test.epub");
    /// # let doc = doc.unwrap();
    /// let id = doc.get_current_id();
    /// assert_eq!("titlepage.xhtml", id.unwrap());
    /// ```
    pub fn get_current_id(&self) -> Result<String, Box<Error>> {
        let current_id = self.spine.get(self.current);
        match current_id {
            Some(id) => return Ok(id.to_string()),
            None => return Err(Box::new(DocError { error: String::from("current is broken") }))
        }
    }

    /// Changes current to the next chapter
    ///
    /// # Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # let doc = EpubDoc::new("test.epub");
    /// # let mut doc = doc.unwrap();
    /// doc.go_next();
    /// assert_eq!("000.xhtml", doc.get_current_id().unwrap());
    ///
    /// let len = doc.spine.len();
    /// for i in 1..len {
    ///     doc.go_next();
    /// }
    /// assert!(doc.go_next().is_err());
    /// ```
    ///
    /// # Errors
    ///
    /// If the page is the last, will not change and an error will be returned
    pub fn go_next(&mut self) -> Result<(), DocError> {
        if self.current + 1 >= self.spine.len() {
            return Err(DocError { error: String::from("last page") });
        }
        self.current += 1;
        Ok(())
    }

    /// Changes current to the prev chapter
    ///
    /// # Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # let doc = EpubDoc::new("test.epub");
    /// # let mut doc = doc.unwrap();
    /// assert!(doc.go_prev().is_err());
    ///
    /// doc.go_next(); // 000.xhtml
    /// doc.go_next(); // 001.xhtml
    /// doc.go_next(); // 002.xhtml
    /// doc.go_prev(); // 001.xhtml
    /// assert_eq!("001.xhtml", doc.get_current_id().unwrap());
    /// ```
    ///
    /// # Errors
    ///
    /// If the page is the first, will not change and an error will be returned
    pub fn go_prev(&mut self) -> Result<(), DocError> {
        if self.current < 1 {
            return Err(DocError { error: String::from("first page") });
        }
        self.current -= 1;
        Ok(())
    }

    /// Returns the number of chapters
    ///
    /// # Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # let doc = EpubDoc::new("test.epub");
    /// # let mut doc = doc.unwrap();
    /// assert_eq!(17, doc.get_num_pages());
    /// ```
    pub fn get_num_pages(&self) -> usize {
        self.spine.len()
    }

    /// Returns the current chapter number, starting from 0
    pub fn get_current_page(&self) -> usize {
        self.current
    }

    /// Changes the current page
    ///
    /// # Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # let doc = EpubDoc::new("test.epub");
    /// # let mut doc = doc.unwrap();
    /// assert_eq!(0, doc.get_current_page());
    /// doc.set_current_page(2);
    /// assert_eq!("001.xhtml", doc.get_current_id().unwrap());
    /// assert_eq!(2, doc.get_current_page());
    /// assert!(doc.set_current_page(50).is_err());
    /// ```
    ///
    /// # Errors
    ///
    /// If the page isn't valid, will not change and an error will be returned
    pub fn set_current_page(&mut self, n: usize) -> Result<(), DocError> {
        if n >= self.spine.len() {
            return Err(DocError { error: String::from("page not valid") });
        }
        self.current = n;
        Ok(())
    }

    fn fill_resources(&mut self) -> Result<(), Box<Error>> {
        let container = try!(self.archive.get_entry(&self.root_file));
        let xml = xmlutils::XMLReader::new(container.as_slice());
        let root = try!(xml.parse_xml());

        // resources from manifest
        let manifest = try!(root.borrow().find("manifest"));
        for r in manifest.borrow().childs.iter() {
            let item = r.borrow();
            let id = try!(item.get_attr("id"));
            let href = try!(item.get_attr("href"));
            let mtype = try!(item.get_attr("media-type"));
            self.resources.insert(id, (self.root_base.to_string() + &href, mtype));
        }

        // items from spine
        let spine = try!(root.borrow().find("spine"));
        for r in spine.borrow().childs.iter() {
            let item = r.borrow();
            let id = try!(item.get_attr("idref"));
            self.spine.push(id);
        }

        // metadata
        let metadata = try!(root.borrow().find("metadata"));
        for r in metadata.borrow().childs.iter() {
            let item = r.borrow();
            if item.name.local_name == "meta" {
                let k = try!(item.get_attr("name"));
                let v = try!(item.get_attr("content"));
                self.metadata.insert(k, v);
            } else {
                let ref k = item.name.local_name;
                let v = match item.text { Some(ref x) => x.to_string(), None => String::from("") };
                self.metadata.insert(k.to_string(), v);
            }
        }

        Ok(())
    }
}

fn get_root_file(container: Vec<u8>) -> Result<String, Box<Error>> {
    let xml = xmlutils::XMLReader::new(container.as_slice());
    let root = try!(xml.parse_xml());
    let el = root.borrow();
    let element = try!(el.find("rootfile"));
    let el2 = element.borrow();

    Ok(try!(el2.get_attr("full-path")))
}
