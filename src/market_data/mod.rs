use crate::types::*;
use crate::orderbook::OrderBook;
use std::sync::{Arc, Mutex};
use tokio::time::{Duration, Instant};

pub struct MarketDataProcessor {
    order_book: Arc<Mutex<OrderBook>>,
    stats: MarketDataStats,
}

impl MarketDataProcessor {
    pub fn new(order_book: Arc<Mutex<OrderBook>>) -> Self {
        Self {
            order_book,
            stats: MarketDataStats::default(),
        }
    }

    pub async fn process_market_data(&mut self, message: MarketDataMessage) -> Result<bool, OrderBookError> {
        let start_time = Instant::now();
        
        let mut order_book = self.order_book.lock().unwrap();
        
        // check sequence number
        let sequence_number = match &message {
            MarketDataMessage::NewOrder(msg) => msg.sequence_number,
            MarketDataMessage::CancelOrder(msg) => msg.sequence_number,
            MarketDataMessage::ModifyOrder(msg) => msg.sequence_number,
            MarketDataMessage::Trade(msg) => msg.sequence_number,
            MarketDataMessage::BookSnapshot(msg) => msg.sequence_number,
        };

        if sequence_number <= order_book.last_sequence_number {
            self.stats.sequence_gaps += 1;
            return Err(OrderBookError::SequenceGap { 
                expected: order_book.last_sequence_number + 1, 
                actual: sequence_number 
            });
        }

        order_book.last_sequence_number = sequence_number;

        // process the message
        match message {
            MarketDataMessage::NewOrder(msg) => {
                let order = Order::new(
                    msg.side,
                    msg.order_type,
                    msg.price,
                    msg.quantity,
                    None,
                );
                order_book.add_order(order)?;
                self.stats.new_orders += 1;
            }
            MarketDataMessage::CancelOrder(msg) => {
                order_book.cancel_order(msg.order_id)?;
                self.stats.cancellations += 1;
            }
            MarketDataMessage::ModifyOrder(msg) => {
                order_book.modify_order(msg.order_id, Some(msg.new_price.unwrap_or(0)), msg.new_quantity)?;
                self.stats.modifications += 1;
            }
            MarketDataMessage::Trade(_msg) => {
                // trade messages are informational - just record them
                self.stats.trades += 1;
            }
            MarketDataMessage::BookSnapshot(msg) => {
                // Update order book with real market data
                self.update_orderbook_with_snapshot(&mut order_book, &msg)?;
                self.stats.snapshots += 1;
            }
        }

        // update statistics
        let processing_time = start_time.elapsed();
        self.stats.total_processing_time += processing_time;
        self.stats.max_latency = self.stats.max_latency.max(processing_time);
        if self.stats.min_latency == Duration::ZERO {
            self.stats.min_latency = processing_time;
        } else {
            self.stats.min_latency = self.stats.min_latency.min(processing_time);
        }
        self.stats.messages_processed += 1;

        Ok(true)
    }

    pub async fn process_market_data_batch(&mut self, messages: Vec<MarketDataMessage>) -> Result<usize, OrderBookError> {
        let mut processed = 0;
        
        for message in messages {
            match self.process_market_data(message).await {
                Ok(_) => processed += 1,
                Err(e) => {
                    self.stats.errors += 1;
                    eprintln!("Error processing market data: {}", e);
                }
            }
        }
        
        Ok(processed)
    }

    pub fn get_stats(&self) -> &MarketDataStats {
        &self.stats
    }

    pub fn reset_stats(&mut self) {
        self.stats.reset();
    }

    fn update_orderbook_with_snapshot(&self, order_book: &mut OrderBook, snapshot: &BookSnapshotMessage) -> Result<(), OrderBookError> {
        // Clear existing orders and rebuild with real market data
        order_book.clear_all_orders();
        
        // Add bids from snapshot
        for (i, bid) in snapshot.bids.iter().enumerate() {
            let order = Order::new(
                Side::Buy,
                OrderType::Limit,
                bid.price,
                bid.quantity,
                Some(format!("market_bid_{}", i)),
            );
            order_book.add_order(order)?;
        }
        
        // Add asks from snapshot
        for (i, ask) in snapshot.asks.iter().enumerate() {
            let order = Order::new(
                Side::Sell,
                OrderType::Limit,
                ask.price,
                ask.quantity,
                Some(format!("market_ask_{}", i)),
            );
            order_book.add_order(order)?;
        }
        
        Ok(())
    }
}

pub struct BinanceMarketDataFeed {
    symbol: String,
    client: reqwest::Client,
}

impl BinanceMarketDataFeed {
    pub fn new(symbol: String) -> Self {
        Self {
            symbol,
            client: reqwest::Client::new(),
        }
    }

    pub async fn get_order_book_snapshot(&self) -> Result<BookSnapshotMessage, Box<dyn std::error::Error>> {
        let url = format!("https://api.binance.com/api/v3/depth?symbol={}&limit=1000", self.symbol);
        let response = self.client.get(&url).send().await?;
        let data: serde_json::Value = response.json().await?;

        let mut bids = Vec::new();
        let mut asks = Vec::new();

        // parse bids
        if let Some(bids_data) = data["bids"].as_array() {
            for bid in bids_data {
                if let (Some(price_str), Some(quantity_str)) = (bid[0].as_str(), bid[1].as_str()) {
                    let price = (price_str.parse::<f64>()? * 1_000_000.0) as u64; // convert to micros
                    let quantity = (quantity_str.parse::<f64>()? * 1_000_000.0) as u64;
                    bids.push(LevelInfo {
                        price,
                        quantity,
                        order_count: 1, // binance doesn't provide order count
                    });
                }
            }
        }

        // parse asks
        if let Some(asks_data) = data["asks"].as_array() {
            for ask in asks_data {
                if let (Some(price_str), Some(quantity_str)) = (ask[0].as_str(), ask[1].as_str()) {
                    let price = (price_str.parse::<f64>()? * 1_000_000.0) as u64; // convert to micros
                    let quantity = (quantity_str.parse::<f64>()? * 1_000_000.0) as u64;
                    asks.push(LevelInfo {
                        price,
                        quantity,
                        order_count: 1, // binance doesn't provide order count
                    });
                }
            }
        }

        Ok(BookSnapshotMessage {
            message_type: MessageType::BookSnapshot,
            bids,
            asks,
            timestamp: chrono::Utc::now(),
            sequence_number: 0, // binance doesn't provide sequence numbers in rest api
        })
    }

    pub async fn start_live_feed(&self, order_book: Arc<Mutex<OrderBook>>, interval_ms: u64) -> Result<(), Box<dyn std::error::Error>> {
        let mut processor = MarketDataProcessor::new(order_book.clone());
        let mut interval = tokio::time::interval(Duration::from_millis(interval_ms));
        let mut sequence_counter = 1u64;

        loop {
            interval.tick().await;
            
            match self.get_order_book_snapshot().await {
                Ok(mut snapshot) => {
                    // Set proper sequence number for REST API data
                    snapshot.sequence_number = sequence_counter;
                    sequence_counter += 1;
                    
                    let message = MarketDataMessage::BookSnapshot(snapshot);
                    if let Err(e) = processor.process_market_data(message).await {
                        eprintln!("Error processing snapshot: {}", e);
                    } else {
                        // Display the live orderbook after successful update
                        let order_book_guard = order_book.lock().unwrap();
                        order_book_guard.display_live_orderbook(&self.symbol, 10);
                        drop(order_book_guard);
                    }
                }
                Err(e) => {
                    eprintln!("Error fetching market data: {}", e);
                }
            }
        }
    }
}
