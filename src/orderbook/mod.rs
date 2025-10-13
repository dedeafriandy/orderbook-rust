use crate::types::*;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

#[derive(Debug, Clone)]
struct OrderEntry {
    order: Arc<Order>,
    price_level_index: usize,
}

#[derive(Debug, Clone)]
struct PriceLevel {
    price: Price,
    orders: VecDeque<Arc<Order>>,
    total_quantity: Quantity,
}

impl PriceLevel {
    fn new(price: Price) -> Self {
        Self {
            price,
            orders: VecDeque::new(),
            total_quantity: 0,
        }
    }

    fn add_order(&mut self, order: Arc<Order>) {
        self.total_quantity += order.remaining_quantity;
        self.orders.push_back(order);
    }

    fn remove_order(&mut self, order_id: OrderId) -> Option<Arc<Order>> {
        if let Some(pos) = self.orders.iter().position(|o| o.id == order_id) {
            let order = self.orders.remove(pos).unwrap();
            self.total_quantity -= order.remaining_quantity;
            Some(order)
        } else {
            None
        }
    }

    fn update_order(&mut self, order_id: OrderId, new_quantity: Quantity) -> Option<Arc<Order>> {
        if let Some(pos) = self.orders.iter().position(|o| o.id == order_id) {
            let order = self.orders.remove(pos).unwrap();
            let old_quantity = order.remaining_quantity;
            self.total_quantity = self.total_quantity - old_quantity + new_quantity;
            
            // create new order with updated quantity
            let mut new_order = (*order).clone();
            new_order.remaining_quantity = new_quantity;
            let new_order_arc = Arc::new(new_order);
            
            self.orders.insert(pos, new_order_arc.clone());
            Some(new_order_arc)
        } else {
            None
        }
    }

    fn get_level_info(&self) -> LevelInfo {
        LevelInfo {
            price: self.price,
            quantity: self.total_quantity,
            order_count: self.orders.len(),
        }
    }
}

pub struct OrderBook {
    // bids: highest price first (descending order)
    bids: BTreeMap<Price, PriceLevel>,
    // asks: lowest price first (ascending order)  
    asks: BTreeMap<Price, PriceLevel>,
    // order lookup: O(1) access to orders
    orders: HashMap<OrderId, OrderEntry>,
    // statistics
    stats: MarketDataStats,
    pub last_sequence_number: u64,
    is_initialized: bool,
    // day reset configuration
    day_reset_hour: u8,
    day_reset_minute: u8,
    last_day_reset: Timestamp,
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            orders: HashMap::new(),
            stats: MarketDataStats::default(),
            last_sequence_number: 0,
            is_initialized: false,
            day_reset_hour: 15,
            day_reset_minute: 59,
            last_day_reset: chrono::Utc::now(),
        }
    }

    pub fn set_day_reset_time(&mut self, hour: u8, minute: u8) {
        self.day_reset_hour = hour;
        self.day_reset_minute = minute;
    }

    pub fn add_order(&mut self, order: Order) -> Result<Vec<Trade>, OrderBookError> {
        let start_time = Instant::now();
        
        // check if order already exists
        if self.orders.contains_key(&order.id) {
            return Err(OrderBookError::OrderAlreadyExists { order_id: order.id });
        }

        // validate order
        if order.price == 0 && order.order_type != OrderType::Market {
            return Err(OrderBookError::InvalidPrice { price: order.price });
        }
        
        if order.quantity == 0 {
            return Err(OrderBookError::InvalidQuantity { quantity: order.quantity });
        }

        let order_arc = Arc::new(order);
        let mut trades = Vec::new();

        // handle different order types
        match order_arc.order_type {
            OrderType::Market => {
                trades = self.match_market_order(&order_arc)?;
            }
            OrderType::FillOrKill => {
                trades = self.match_fill_or_kill(&order_arc)?;
            }
            OrderType::ImmediateOrCancel => {
                trades = self.match_immediate_or_cancel(&order_arc)?;
            }
            _ => {
                // limit orders, gtc, gfd - add to book and try to match
                trades = self.add_to_book_and_match(&order_arc)?;
            }
        }

        // update statistics
        let processing_time = start_time.elapsed();
        self.stats.total_processing_time += processing_time;
        self.stats.max_latency = self.stats.max_latency.max(processing_time);
        if self.stats.min_latency == std::time::Duration::ZERO {
            self.stats.min_latency = processing_time;
        } else {
            self.stats.min_latency = self.stats.min_latency.min(processing_time);
        }
        self.stats.new_orders += 1;

        Ok(trades)
    }

    pub fn cancel_order(&mut self, order_id: OrderId) -> Result<(), OrderBookError> {
        let start_time = Instant::now();
        
        let order_entry = self.orders.remove(&order_id)
            .ok_or(OrderBookError::OrderNotFound { order_id })?;

        let order = &order_entry.order;
        
        match order.side {
            Side::Buy => {
                if let Some(price_level) = self.bids.get_mut(&order.price) {
                    price_level.remove_order(order_id);
                    if price_level.orders.is_empty() {
                        self.bids.remove(&order.price);
                    }
                }
            }
            Side::Sell => {
                if let Some(price_level) = self.asks.get_mut(&order.price) {
                    price_level.remove_order(order_id);
                    if price_level.orders.is_empty() {
                        self.asks.remove(&order.price);
                    }
                }
            }
        }

        // update statistics
        let processing_time = start_time.elapsed();
        self.stats.total_processing_time += processing_time;
        self.stats.cancellations += 1;

        Ok(())
    }

    pub fn modify_order(&mut self, order_id: OrderId, new_price: Option<Price>, new_quantity: Option<Quantity>) -> Result<Vec<Trade>, OrderBookError> {
        let start_time = Instant::now();
        
        let order_entry = self.orders.get(&order_id)
            .ok_or(OrderBookError::OrderNotFound { order_id })?
            .clone();

        let old_order = order_entry.order.clone();
        
        // cancel the old order
        self.cancel_order(order_id)?;
        
        // create new order with modifications
        let mut new_order = (*old_order).clone();
        if let Some(price) = new_price {
            new_order.price = price;
        }
        if let Some(quantity) = new_quantity {
            new_order.remaining_quantity = quantity;
        }
        new_order.id = Uuid::new_v4(); // new id for the modified order
        new_order.timestamp = chrono::Utc::now();

        // add the new order
        let trades = self.add_order(new_order)?;

        // update statistics
        let processing_time = start_time.elapsed();
        self.stats.total_processing_time += processing_time;
        self.stats.modifications += 1;

        Ok(trades)
    }

    pub fn size(&self) -> usize {
        self.orders.len()
    }

    pub fn get_order_book_snapshot(&self, max_levels: usize) -> OrderBookSnapshot {
        let mut bids = Vec::new();
        let mut asks = Vec::new();

        // get best bids (highest prices first)
        for (_, price_level) in self.bids.iter().rev().take(max_levels) {
            bids.push(price_level.get_level_info());
        }

        // get best asks (lowest prices first)
        for (_, price_level) in self.asks.iter().take(max_levels) {
            asks.push(price_level.get_level_info());
        }

        OrderBookSnapshot {
            bids,
            asks,
            timestamp: chrono::Utc::now(),
            sequence_number: self.last_sequence_number,
        }
    }

    pub fn get_best_bid(&self) -> Option<Price> {
        self.bids.keys().next_back().copied()
    }

    pub fn get_best_ask(&self) -> Option<Price> {
        self.asks.keys().next().copied()
    }

    pub fn get_market_data_stats(&self) -> &MarketDataStats {
        &self.stats
    }

    pub fn reset_market_data_stats(&mut self) {
        self.stats.reset();
    }

    pub fn is_initialized(&self) -> bool {
        self.is_initialized
    }

    pub fn get_last_sequence_number(&self) -> u64 {
        self.last_sequence_number
    }

    // private helper methods
    fn add_to_book_and_match(&mut self, order: &Arc<Order>) -> Result<Vec<Trade>, OrderBookError> {
        let mut trades = Vec::new();
        
        // try to match first
        trades = self.match_orders(order)?;
        
        // calculate remaining quantity after trades
        let total_traded: Quantity = trades.iter().map(|t| t.quantity).sum();
        let remaining_quantity = order.remaining_quantity - total_traded;
        
        // if order still has remaining quantity, add to book
        if remaining_quantity > 0 {
            // create a new order with the remaining quantity
            let mut remaining_order = order.as_ref().clone();
            remaining_order.remaining_quantity = remaining_quantity;
            self.add_to_book(&Arc::new(remaining_order))?;
        }
        
        Ok(trades)
    }

    fn add_to_book(&mut self, order: &Arc<Order>) -> Result<(), OrderBookError> {
        let order_entry = OrderEntry {
            order: order.clone(),
            price_level_index: 0, // will be updated when needed
        };

        self.orders.insert(order.id, order_entry);

        match order.side {
            Side::Buy => {
                self.bids.entry(order.price)
                    .or_insert_with(|| PriceLevel::new(order.price))
                    .add_order(order.clone());
            }
            Side::Sell => {
                self.asks.entry(order.price)
                    .or_insert_with(|| PriceLevel::new(order.price))
                    .add_order(order.clone());
            }
        }

        Ok(())
    }

    fn match_orders(&mut self, incoming_order: &Arc<Order>) -> Result<Vec<Trade>, OrderBookError> {
        let mut trades = Vec::new();
        let mut remaining_quantity = incoming_order.remaining_quantity;

        match incoming_order.side {
            Side::Buy => {
                // match against asks (sell orders)
                while remaining_quantity > 0 {
                    let best_ask_price = if let Some((&price, _)) = self.asks.first_key_value() {
                        price
                    } else {
                        break;
                    };
                    
                    // for buy orders, match if our price is >= ask price
                    if incoming_order.price >= best_ask_price || incoming_order.order_type == OrderType::Market {
                        let trade = self.match_at_price_level_ask(best_ask_price, incoming_order, remaining_quantity)?;
                        remaining_quantity -= trade.quantity;
                        trades.push(trade);
                    } else {
                        break;
                    }
                }
            }
            Side::Sell => {
                // match against bids (buy orders)
                while remaining_quantity > 0 {
                    let best_bid_price = if let Some((&price, _)) = self.bids.last_key_value() {
                        price
                    } else {
                        break;
                    };
                    
                    // for sell orders, match if our price is <= bid price
                    if incoming_order.price <= best_bid_price || incoming_order.order_type == OrderType::Market {
                        let trade = self.match_at_price_level_bid(best_bid_price, incoming_order, remaining_quantity)?;
                        remaining_quantity -= trade.quantity;
                        trades.push(trade);
                    } else {
                        break;
                    }
                }
            }
        }

        Ok(trades)
    }

    fn match_at_price_level_ask(&mut self, price: Price, incoming_order: &Arc<Order>, max_quantity: Quantity) -> Result<Trade, OrderBookError> {
        if let Some(price_level) = self.asks.get_mut(&price) {
            if let Some(resting_order) = price_level.orders.front() {
                let trade_quantity = max_quantity.min(resting_order.remaining_quantity);
                let trade_price = resting_order.price; // use resting order's price
                
                let trade = Trade::new(
                    incoming_order.id,
                    resting_order.id,
                    trade_price,
                    trade_quantity,
                );

                // update quantities
                price_level.total_quantity -= trade_quantity;

                // remove filled orders or update remaining quantity
                if resting_order.remaining_quantity == trade_quantity {
                    price_level.orders.pop_front();
                } else {
                    // create new order with updated quantity
                    let mut updated_order = resting_order.as_ref().clone();
                    updated_order.remaining_quantity -= trade_quantity;
                    price_level.orders.pop_front();
                    price_level.orders.push_front(Arc::new(updated_order));
                }

                // remove empty price levels
                if price_level.orders.is_empty() {
                    self.asks.remove(&price);
                }

                Ok(trade)
            } else {
                Err(OrderBookError::OrderNotFound { order_id: incoming_order.id })
            }
        } else {
            Err(OrderBookError::OrderNotFound { order_id: incoming_order.id })
        }
    }

    fn match_at_price_level_bid(&mut self, price: Price, incoming_order: &Arc<Order>, max_quantity: Quantity) -> Result<Trade, OrderBookError> {
        if let Some(price_level) = self.bids.get_mut(&price) {
            if let Some(resting_order) = price_level.orders.front() {
                let trade_quantity = max_quantity.min(resting_order.remaining_quantity);
                let trade_price = resting_order.price; // use resting order's price
                
                let trade = Trade::new(
                    resting_order.id,
                    incoming_order.id,
                    trade_price,
                    trade_quantity,
                );

                // update quantities
                price_level.total_quantity -= trade_quantity;

                // remove filled orders or update remaining quantity
                if resting_order.remaining_quantity == trade_quantity {
                    price_level.orders.pop_front();
                } else {
                    // create new order with updated quantity
                    let mut updated_order = resting_order.as_ref().clone();
                    updated_order.remaining_quantity -= trade_quantity;
                    price_level.orders.pop_front();
                    price_level.orders.push_front(Arc::new(updated_order));
                }

                // remove empty price levels
                if price_level.orders.is_empty() {
                    self.bids.remove(&price);
                }

                Ok(trade)
            } else {
                Err(OrderBookError::OrderNotFound { order_id: incoming_order.id })
            }
        } else {
            Err(OrderBookError::OrderNotFound { order_id: incoming_order.id })
        }
    }

    fn match_market_order(&mut self, order: &Arc<Order>) -> Result<Vec<Trade>, OrderBookError> {
        // convert market order to aggressive limit order
        let aggressive_price = match order.side {
            Side::Buy => Price::MAX,  // buy at any price
            Side::Sell => 0,          // sell at any price
        };

        let mut market_order = order.as_ref().clone();
        market_order.price = aggressive_price;
        let market_order_arc = Arc::new(market_order);

        self.match_orders(&market_order_arc)
    }

    fn match_fill_or_kill(&mut self, order: &Arc<Order>) -> Result<Vec<Trade>, OrderBookError> {
        // check if we can fill the entire order
        let available_quantity = self.get_available_quantity_for_order(order);
        
        if available_quantity >= order.quantity {
            self.match_orders(order)
        } else {
            // reject the order
            Ok(Vec::new())
        }
    }

    fn match_immediate_or_cancel(&mut self, order: &Arc<Order>) -> Result<Vec<Trade>, OrderBookError> {
        // match what we can, cancel the rest
        self.match_orders(order)
    }

    fn get_available_quantity_for_order(&self, order: &Arc<Order>) -> Quantity {
        match order.side {
            Side::Buy => {
                self.asks.values()
                    .filter(|level| level.price <= order.price)
                    .map(|level| level.total_quantity)
                    .sum()
            }
            Side::Sell => {
                self.bids.values()
                    .filter(|level| level.price >= order.price)
                    .map(|level| level.total_quantity)
                    .sum()
            }
        }
    }

    /// Clear all orders from the order book (useful for rebuilding with market data)
    pub fn clear_all_orders(&mut self) {
        self.bids.clear();
        self.asks.clear();
        self.orders.clear();
        self.stats.reset();
    }

    /// Display the orderbook in a tabular format similar to the C++ project
    pub fn display_live_orderbook(&self, symbol: &str, max_levels: usize) {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S");
        
        println!("\n{}", "=".repeat(80));
        println!("LIVE ORDERBOOK: {}", symbol);
        println!("{}", timestamp);
        println!("{}", "=".repeat(80));
        
        // Get top levels from both sides
        let mut bid_levels: Vec<_> = self.bids.iter().rev().take(max_levels).collect();
        let mut ask_levels: Vec<_> = self.asks.iter().take(max_levels).collect();
        
        // Create empty price level for padding
        let empty_price_level = PriceLevel::new(0);
        
        // Pad with empty levels if needed
        while bid_levels.len() < max_levels {
            bid_levels.push((&0, &empty_price_level));
        }
        while ask_levels.len() < max_levels {
            ask_levels.push((&0, &empty_price_level));
        }
        
        // Header
        println!("{:<12} | {:<12} | {:<12} | {:<12}", "BID QTY", "BID PRICE", "ASK PRICE", "ASK QTY");
        println!("{}", "-".repeat(80));
        
        // Display levels
        for i in 0..max_levels {
            let (bid_price, bid_level) = bid_levels[i];
            let (ask_price, ask_level) = ask_levels[i];
            
            let bid_qty = if *bid_price > 0 { 
                format!("{:.2}", bid_level.total_quantity as f64 / 1_000_000.0) 
            } else { 
                "".to_string() 
            };
            
            let bid_price_str = if *bid_price > 0 { 
                format!("${:.2}", *bid_price as f64 / 1_000_000.0) 
            } else { 
                "".to_string() 
            };
            
            let ask_price_str = if *ask_price > 0 { 
                format!("${:.2}", *ask_price as f64 / 1_000_000.0) 
            } else { 
                "".to_string() 
            };
            
            let ask_qty = if *ask_price > 0 { 
                format!("{:.2}", ask_level.total_quantity as f64 / 1_000_000.0) 
            } else { 
                "".to_string() 
            };
            
            println!("{:<12} | {:<12} | {:<12} | {:<12}", bid_qty, bid_price_str, ask_price_str, ask_qty);
        }
        
        // Footer with best bid/ask and spread
        println!("{}", "=".repeat(80));
        if let Some(best_bid) = self.get_best_bid() {
            if let Some(best_ask) = self.get_best_ask() {
                let spread = best_ask - best_bid;
                let spread_bps = (spread as f64 / best_bid as f64) * 10000.0;
                println!("Best Bid: ${:.2} | Best Ask: ${:.2} | Spread: ${:.2} ({:.1} bps)", 
                    best_bid as f64 / 1_000_000.0,
                    best_ask as f64 / 1_000_000.0,
                    spread as f64 / 1_000_000.0,
                    spread_bps
                );
            } else {
                println!("Best Bid: ${:.2} | Best Ask: None", best_bid as f64 / 1_000_000.0);
            }
        } else {
            println!("Best Bid: None | Best Ask: None");
        }
        println!("{}", "=".repeat(80));
    }
}

impl Default for OrderBook {
    fn default() -> Self {
        Self::new()
    }
}
