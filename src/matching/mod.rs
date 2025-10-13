use crate::types::*;
use std::collections::HashMap;

pub struct MatchingEngine {
    order_books: HashMap<String, crate::orderbook::OrderBook>,
}

impl MatchingEngine {
    pub fn new() -> Self {
        Self {
            order_books: HashMap::new(),
        }
    }

    pub fn add_order(&mut self, symbol: &str, order: Order) -> Result<Vec<Trade>, OrderBookError> {
        let order_book = self.order_books.entry(symbol.to_string()).or_insert_with(|| crate::orderbook::OrderBook::new());
        order_book.add_order(order)
    }

    pub fn cancel_order(&mut self, symbol: &str, order_id: OrderId) -> Result<(), OrderBookError> {
        if let Some(order_book) = self.order_books.get_mut(symbol) {
            order_book.cancel_order(order_id)
        } else {
            Err(OrderBookError::OrderNotFound { order_id })
        }
    }

    pub fn modify_order(&mut self, symbol: &str, order_id: OrderId, new_price: Option<Price>, new_quantity: Option<Quantity>) -> Result<Vec<Trade>, OrderBookError> {
        if let Some(order_book) = self.order_books.get_mut(symbol) {
            order_book.modify_order(order_id, new_price, new_quantity)
        } else {
            Err(OrderBookError::OrderNotFound { order_id })
        }
    }

    pub fn get_order_book_snapshot(&self, symbol: &str, max_levels: usize) -> Option<OrderBookSnapshot> {
        self.order_books.get(symbol).map(|ob| ob.get_order_book_snapshot(max_levels))
    }

    pub fn get_best_bid_ask(&self, symbol: &str) -> Option<(Option<Price>, Option<Price>)> {
        self.order_books.get(symbol).map(|ob| (ob.get_best_bid(), ob.get_best_ask()))
    }

    pub fn get_symbols(&self) -> Vec<String> {
        self.order_books.keys().cloned().collect()
    }
}

impl Default for MatchingEngine {
    fn default() -> Self {
        Self::new()
    }
}
