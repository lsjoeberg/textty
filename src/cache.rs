use crate::texttv::{HOME_PAGE_NR, MAX_PAGE_NR};
use crate::tui::{PageContext, PageSet};

const PAGE_COUNT: usize = (MAX_PAGE_NR - HOME_PAGE_NR) as usize;

pub struct Cache<'a> {
    pages: [Option<PageContext<'a>>; PAGE_COUNT],
}

impl<'a> Cache<'a> {
    pub fn new() -> Self {
        let pages = [const { None }; PAGE_COUNT];
        Self { pages }
    }

    pub fn get(&self, num: u16) -> Option<&PageContext<'a>> {
        let idx = num.saturating_sub(HOME_PAGE_NR) as usize;
        self.pages.get(idx).and_then(|x| x.as_ref())
    }

    pub fn store(&mut self, item: PageContext<'a>) {
        let idx = item.this_nr.saturating_sub(HOME_PAGE_NR) as usize;
        self.pages[idx] = Some(item);
    }
}
