use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type OrderId = Uuid;
pub type Price = u64; // using u64 for microsecond precision
pub type Quantity = u64;
pub type Timestamp = chrono::DateTime<chrono::Utc>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrderType {
    Limit,
    Market,
    ImmediateOrCancel, // ioc
    FillOrKill,        // fok
    GoodTillCancel,    // gtc
    GoodForDay,        // gfd
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: OrderId,
    pub side: Side,
    pub order_type: OrderType,
    pub price: Price,
    pub quantity: Quantity,
    pub remaining_quantity: Quantity,
    pub timestamp: Timestamp,
    pub user_id: Option<String>,
}

impl Order {
    pub fn new(
        side: Side,
        order_type: OrderType,
        price: Price,
        quantity: Quantity,
        user_id: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            side,
            order_type,
            price,
            quantity,
            remaining_quantity: quantity,
            timestamp: chrono::Utc::now(),
            user_id,
        }
    }

    pub fn is_filled(&self) -> bool {
        self.remaining_quantity == 0
    }

    pub fn is_active(&self) -> bool {
        !self.is_filled() && self.order_type != OrderType::FillOrKill
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub id: OrderId,
    pub buy_order_id: OrderId,
    pub sell_order_id: OrderId,
    pub price: Price,
    pub quantity: Quantity,
    pub timestamp: Timestamp,
}

impl Trade {
    pub fn new(
        buy_order_id: OrderId,
        sell_order_id: OrderId,
        price: Price,
        quantity: Quantity,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            buy_order_id,
            sell_order_id,
            price,
            quantity,
            timestamp: chrono::Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelInfo {
    pub price: Price,
    pub quantity: Quantity,
    pub order_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookSnapshot {
    pub bids: Vec<LevelInfo>,
    pub asks: Vec<LevelInfo>,
    pub timestamp: Timestamp,
    pub sequence_number: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    NewOrder,
    CancelOrder,
    ModifyOrder,
    Trade,
    BookSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewOrderMessage {
    pub message_type: MessageType,
    pub order_id: OrderId,
    pub side: Side,
    pub order_type: OrderType,
    pub price: Price,
    pub quantity: Quantity,
    pub timestamp: Timestamp,
    pub sequence_number: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelOrderMessage {
    pub message_type: MessageType,
    pub order_id: OrderId,
    pub timestamp: Timestamp,
    pub sequence_number: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifyOrderMessage {
    pub message_type: MessageType,
    pub order_id: OrderId,
    pub new_price: Option<Price>,
    pub new_quantity: Option<Quantity>,
    pub timestamp: Timestamp,
    pub sequence_number: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeMessage {
    pub message_type: MessageType,
    pub buy_order_id: OrderId,
    pub sell_order_id: OrderId,
    pub price: Price,
    pub quantity: Quantity,
    pub timestamp: Timestamp,
    pub sequence_number: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookSnapshotMessage {
    pub message_type: MessageType,
    pub bids: Vec<LevelInfo>,
    pub asks: Vec<LevelInfo>,
    pub timestamp: Timestamp,
    pub sequence_number: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketDataMessage {
    NewOrder(NewOrderMessage),
    CancelOrder(CancelOrderMessage),
    ModifyOrder(ModifyOrderMessage),
    Trade(TradeMessage),
    BookSnapshot(BookSnapshotMessage),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MarketDataStats {
    pub messages_processed: u64,
    pub new_orders: u64,
    pub cancellations: u64,
    pub modifications: u64,
    pub trades: u64,
    pub snapshots: u64,
    pub errors: u64,
    pub sequence_gaps: u64,
    pub total_processing_time: std::time::Duration,
    pub max_latency: std::time::Duration,
    pub min_latency: std::time::Duration,
}

impl MarketDataStats {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn get_average_latency_micros(&self) -> f64 {
        if self.messages_processed > 0 {
            self.total_processing_time.as_micros() as f64 / self.messages_processed as f64
        } else {
            0.0
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OrderBookError {
    #[error("Order not found: {order_id}")]
    OrderNotFound { order_id: OrderId },
    
    #[error("Invalid price: {price}")]
    InvalidPrice { price: Price },
    
    #[error("Invalid quantity: {quantity}")]
    InvalidQuantity { quantity: Quantity },
    
    #[error("Order already exists: {order_id}")]
    OrderAlreadyExists { order_id: OrderId },
    
    #[error("Sequence gap detected: expected {expected}, got {actual}")]
    SequenceGap { expected: u64, actual: u64 },
    
    #[error("Market data error: {message}")]
    MarketDataError { message: String },
}
