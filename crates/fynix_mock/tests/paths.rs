//! Walks a derived element tree, across modules, through an
//! `Option`.

mod icon {
    use fynix_mock::lenz::Lenz;

    #[derive(Lenz)]
    pub struct Icon {
        pub size: u32,
    }
}

mod badge {
    use fynix_mock::lenz::Lenz;

    use crate::icon::Icon;

    #[derive(Lenz)]
    pub struct Badge {
        /// The optional link, second from the end of the walk.
        pub icon: Option<Icon>,
    }
}

mod header {
    use fynix_mock::lenz::Lenz;

    use crate::badge::Badge;

    #[derive(Lenz)]
    pub struct Header {
        pub badge: Badge,
    }
}

mod card {
    use fynix_mock::lenz::Lenz;

    use crate::header::Header;

    #[derive(Lenz)]
    pub struct Card {
        pub header: Header,
        pub title: String,
    }
}

use badge::{Badge, BadgeCursor};
use card::{Card, CardCursor};
use header::{Header, HeaderCursor};
use icon::{Icon, IconCursor};

fn card() -> Card {
    Card {
        header: Header {
            badge: Badge {
                icon: Some(Icon { size: 12 }),
            },
        },
        title: "Legendary".into(),
    }
}

#[test]
fn reads_through_four_hops_and_four_modules() {
    let c = card();
    let size =
        Card::cursor().header().badge().icon().size().accessor();

    assert_eq!(size.get(&c), Some(&12));
}

#[test]
fn writes_through_the_same_path() {
    let mut card = card();
    let size =
        Card::cursor().header().badge().icon().size().accessor();

    *size.get_mut(&mut card).unwrap() = 16;

    assert_eq!(size.get(&card), Some(&16));
}

#[test]
fn absent_link_short_circuits() {
    let mut card = card();
    let size =
        Card::cursor().header().badge().icon().size().accessor();

    card.header.badge.icon = None;

    assert_eq!(size.get(&card), None);
    assert!(size.get_mut(&mut card).is_none());
}

#[test]
fn leaf_on_the_root_element() {
    let card = card();
    let title = Card::cursor().title().accessor();

    assert_eq!(
        title.get(&card).map(String::as_str),
        Some("Legendary")
    );
}

#[test]
fn accessors_are_plain_copyable_data() {
    let size =
        Card::cursor().header().badge().icon().size().accessor();
    let copied = size;

    let card = card();
    assert_eq!(copied.get(&card), size.get(&card));
}
