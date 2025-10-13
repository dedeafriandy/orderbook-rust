use orderbook_rust::*;
use std::sync::{Arc, Mutex};
use tokio;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Use real market data from Binance instead of demo data
    #[arg(long)]
    real: bool,
    
    /// Trading symbol for real market data (default: BTCUSDT)
    #[arg(long, default_value = "BTCUSDT")]
    symbol: String,
    
    /// Update interval in milliseconds for real market data (default: 1000)
    #[arg(long, default_value = "1000")]
    interval: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    println!("orderBook rust - trading engine");
    
    // create order book
    let order_book = Arc::new(Mutex::new(OrderBook::new()));
    
    // example: add some orders
    let mut order_book_guard = order_book.lock().unwrap();
    
    // add a buy order
    let buy_order = Order::new(
        Side::Buy,
        OrderType::Limit,
        100_000, // $100.00 in micros
        1000,    // 1000 shares
        Some("user1".to_string()),
    );
    
    println!(" adding buy order: {} shares at ${:.2}", 
             buy_order.quantity, 
             buy_order.price as f64 / 1_000_000.0);
    
    let trades = order_book_guard.add_order(buy_order)?;
    println!(" buy order added. Trades executed: {}", trades.len());
    
    // add a sell order
    let sell_order = Order::new(
        Side::Sell,
        OrderType::Limit,
        99_000, // $99.00 in micros
        500,    // 500 shares
        Some("user2".to_string()),
    );
    
    println!(" adding sell order: {} shares at ${:.2}", 
             sell_order.quantity, 
             sell_order.price as f64 / 1_000_000.0);
    
    let trades = order_book_guard.add_order(sell_order)?;
    println!(" sell order added. Trades executed: {}", trades.len());
    
    // get order book snapshot
    let snapshot = order_book_guard.get_order_book_snapshot(5);
    println!("\n order Book Snapshot:");
    println!("Bids ({} levels):", snapshot.bids.len());
    for bid in &snapshot.bids {
        println!("  ${:.2} - {} shares ({} orders)", 
                 bid.price as f64 / 1_000_000.0, 
                 bid.quantity, 
                 bid.order_count);
    }
    
    println!(" asks ({} levels):", snapshot.asks.len());
    for ask in &snapshot.asks {
        println!("  ${:.2} - {} shares ({} orders)", 
                 ask.price as f64 / 1_000_000.0, 
                 ask.quantity, 
                 ask.order_count);
    }
    
    // show best bid/ask
    if let Some(best_bid) = order_book_guard.get_best_bid() {
        println!("\n best Bid: ${:.2}", best_bid as f64 / 1_000_000.0);
    }
    
    if let Some(best_ask) = order_book_guard.get_best_ask() {
        println!(" best Ask: ${:.2}", best_ask as f64 / 1_000_000.0);
    }
    
    // show statistics
    let stats = order_book_guard.get_market_data_stats();
    println!("\n statistics:");
    println!("  orders processed: {}", stats.new_orders);
    println!("  average latency: {:.2} μs", stats.get_average_latency_micros());
    println!("  max latency: {:.2} μs", stats.max_latency.as_micros());
    
    drop(order_book_guard);
    
    // Check if user wants real market data
    if args.real {
        println!("\nStarting live market data feed from Binance...");
        println!("Symbol: {}", args.symbol);
        println!("Update interval: {}ms", args.interval);
        println!("Press Ctrl+C to stop\n");
        
        let feed = BinanceMarketDataFeed::new(args.symbol);
        feed.start_live_feed(order_book, args.interval).await?;
    } else {
        println!("\nTo use real market data, run with: cargo run -- --real");
        println!("Example: cargo run -- --real --symbol ETHUSDT --interval 2000");
    }
    
    Ok(())
}
