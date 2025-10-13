# OrderBook Rust - High Performance Trading Engine

A high-performance orderbook engine implemented in Rust, inspired by the [orderbook-simulator-cpp](https://github.com/SLMolenaar/orderbook-simulator-cpp) project. Designed to handle orders with microsecond latency, multiple order types, and price-time priority matching.

## Features

- **High Performance**: Microsecond latency for critical operations
- **Multiple Order Types**: Limit, Market, IOC, FOK, GTC, GFD
- **Smart Matching**: Price-time priority (FIFO within each price level)
- **Market Data Integration**: Support for Binance feeds
- **Thread-Safe**: Designed for concurrent use
- **Real-time Statistics**: Latency and throughput monitoring

## Supported Order Types

| Type | Description |
|------|-------------|
| **Limit** | Order limited to a specific price |
| **Market** | Order executed at the best available price |
| **IOC** (Immediate or Cancel) | Executed immediately or cancelled |
| **FOK** (Fill or Kill) | Executed completely or rejected |
| **GTC** (Good Till Cancel) | Valid until cancelled |
| **GFD** (Good For Day) | Valid until end of day |

## Architecture

```
OrderBook
├── bids: BTreeMap<Price, PriceLevel>     // Best buy prices first
├── asks: BTreeMap<Price, PriceLevel>     // Best sell prices first
└── orders: HashMap<OrderId, OrderEntry>  // O(1) order lookup
```

### Price Levels
Price levels are stored in ordered maps for efficient access to best bid/ask. Within each level, orders are maintained in a FIFO queue for temporal priority.

### Matching Logic
Orders are matched when:
- Buy price ≥ Best sell price, or
- Sell price ≤ Best buy price

Matching proceeds with price-time priority:
1. Best price levels first
2. Within a level, earlier orders first (FIFO)
3. Partial fills supported for all types except FillOrKill

## Quick Start

```rust
use orderbook_rust::*;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create order book
    let order_book = Arc::new(Mutex::new(OrderBook::new()));
    
    // Add buy order
    let buy_order = Order::new(
        Side::Buy,
        OrderType::Limit,
        100_000, // $100.00 in micros
        1000,    // 1000 shares
        Some("user1".to_string()),
    );
    
    let mut order_book_guard = order_book.lock().unwrap();
    let trades = order_book_guard.add_order(buy_order)?;
    
    // Get order book snapshot
    let snapshot = order_book_guard.get_order_book_snapshot(5);
    println!("Best bid: ${:.2}", snapshot.bids[0].price as f64 / 1_000_000.0);
    
    Ok(())
}
```

## Performance

| Operation | Complexity | Measured Throughput |
|-----------|------------|-------------------|
| Add Order | O(log n) | ~400K ops/sec |
| Cancel Order | O(1) | ~2M ops/sec |
| Modify Order | O(log n) | ~270K ops/sec |
| Matching | O(k log n) | ~350K matches/sec |
| Snapshot | O(m) | ~500K snapshots/sec |

*n = number of price levels, k = number of matches, m = number of orders*

## Testing

```bash
# Run all tests
cargo test

# Run with detailed output
cargo test -- --nocapture

# Run specific tests
cargo test test_order_matching
```

## Dependencies

- **serde**: Serialization/deserialization
- **tokio**: Async runtime
- **reqwest**: HTTP client for market feeds
- **chrono**: Date and time handling
- **uuid**: Unique ID generation
- **thiserror**: Error handling
- **anyhow**: Simplified error handling

## Market Data Integration

```rust
use orderbook_rust::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let order_book = Arc::new(Mutex::new(OrderBook::new()));
    let feed = BinanceMarketDataFeed::new("BTCUSDT".to_string());
    
    // Start live feed (update every 1000ms)
    feed.start_live_feed(order_book, 1000).await?;
    
    Ok(())
}
```

## Configuration

### Daily Reset
```rust
let mut order_book = OrderBook::new();
order_book.set_day_reset_time(15, 59); // 15:59 UTC
```

### Statistics
```rust
let stats = order_book.get_market_data_stats();
println!("Average latency: {:.2} μs", stats.get_average_latency_micros());
println!("Orders processed: {}", stats.new_orders);
```

## Use Cases

- **Algorithmic strategy backtesting**
- **Market microstructure research**
- **Order routing simulation**
- **Exchange matching engine prototyping**
- **Educational tool for understanding orderbooks**

## Current Limitations

- Single instrument support (no multi-asset)
- No persistence layer
- Live data uses polling instead of WebSocket streaming
- Synthetic order IDs for aggregated book levels
- No regulatory compliance features

## Future Improvements

- WebSocket feed integration for lower latency
- Position and risk management
- Multiple matching algorithms (pro-rata, size pro-rata)
- Advanced order types (iceberg, etc.)
- Historical data replay and backtesting framework
- Market impact modeling

## License

This project is licensed under the MIT License. See the LICENSE file for details.

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Support

If you have questions or find bugs, please open an issue on GitHub.