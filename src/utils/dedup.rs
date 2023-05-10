pub struct Dedup<Iter, Item> {
    prev: Item,
    iter: Iter,
}

impl<Iter, Item> Dedup<Iter, Item>
where
    Item: Default,
{
    fn new(iter: Iter) -> Dedup<Iter, Item> {
        Dedup {
            prev: Item::default(),
            iter,
        }
    }
}

impl<Iter, Item> Iterator for Dedup<Iter, Item>
where
    Item: PartialEq,
    Item: ToOwned<Owned = Item>,
    Iter: Iterator<Item = Item>,
{
    type Item = Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.find(|current| match *current != self.prev {
            true => {
                self.prev = current.to_owned();
                true
            }
            false => false,
        })
    }
}

pub trait ToDedup {
    type Item;

    fn dedup(self) -> Dedup<Self, Self::Item>
    where
        Self: Sized;
}

impl<Iter, Item> ToDedup for Iter
where
    Item: Default,
    Item: PartialEq,
    Item: ToOwned<Owned = Item>,
    Iter: Iterator<Item = Item>,
{
    type Item = Iter::Item;

    fn dedup(self) -> Dedup<Self, Self::Item>
    where
        Self: Sized,
    {
        Dedup::new(self)
    }
}
