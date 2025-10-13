#[cfg(test)]
mod tests {
    use crate::*;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    #[test]
    fn test_basic_order_placement() {
        let mut order_book = OrderBook::new();
        
        let order = Order::new(
            Side::Buy,
            OrderType::Limit,
            100_000, // $100.00
            1000,    // 1000 shares
            Some("user1".to_string()),
        );
        
        let trades = order_book.add_order(order).unwrap();
        assert_eq!(trades.len(), 0); // no trades should execute
        
        let snapshot = order_book.get_order_book_snapshot(5);
        assert_eq!(snapshot.bids.len(), 1);
        assert_eq!(snapshot.bids[0].price, 100_000);
        assert_eq!(snapshot.bids[0].quantity, 1000);
    }

    #[test]
    fn test_order_matching() {
        let mut order_book = OrderBook::new();
        
        // add buy order
        let buy_order = Order::new(
            Side::Buy,
            OrderType::Limit,
            100_000, // $100.00
            1000,    // 1000 shares
            Some("user1".to_string()),
        );
        
        let trades = order_book.add_order(buy_order).unwrap();
        assert_eq!(trades.len(), 0);
        
        // add sell order that should match
        let sell_order = Order::new(
            Side::Sell,
            OrderType::Limit,
            99_000, // $99.00 (better price)
            500,    // 500 shares
            Some("user2".to_string()),
        );
        
        let trades = order_book.add_order(sell_order).unwrap();
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].quantity, 500);
        assert_eq!(trades[0].price, 100_000); // should match at buy price
        
        // check remaining order book
        let snapshot = order_book.get_order_book_snapshot(5);
        assert_eq!(snapshot.bids.len(), 1);
        assert_eq!(snapshot.bids[0].quantity, 500); // remaining buy quantity
        assert_eq!(snapshot.asks.len(), 0); // no asks remaining
    }

    #[test]
    fn test_order_cancellation() {
        let mut order_book = OrderBook::new();
        
        let order = Order::new(
            Side::Buy,
            OrderType::Limit,
            100_000,
            1000,
            Some("user1".to_string()),
        );
        
        let order_id = order.id;
        order_book.add_order(order).unwrap();
        
        // cancel the order
        order_book.cancel_order(order_id).unwrap();
        
        let snapshot = order_book.get_order_book_snapshot(5);
        assert_eq!(snapshot.bids.len(), 0);
        assert_eq!(order_book.size(), 0);
    }

    #[test]
    fn test_market_order() {
        let mut order_book = OrderBook::new();
        
        // add a sell order first
        let sell_order = Order::new(
            Side::Sell,
            OrderType::Limit,
            100_000,
            500,
            Some("user1".to_string()),
        );
        
        order_book.add_order(sell_order).unwrap();
        
        // add a market buy order
        let market_buy = Order::new(
            Side::Buy,
            OrderType::Market,
            0, // price doesn't matter for market orders
            300,
            Some("user2".to_string()),
        );
        
        let trades = order_book.add_order(market_buy).unwrap();
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].quantity, 300);
        
        // check remaining order book
        let snapshot = order_book.get_order_book_snapshot(5);
        assert_eq!(snapshot.asks.len(), 1);
        assert_eq!(snapshot.asks[0].quantity, 200); // remaining sell quantity
    }

    #[test]
    fn test_fill_or_kill_order() {
        let mut order_book = OrderBook::new();
        
        // add a sell order
        let sell_order = Order::new(
            Side::Sell,
            OrderType::Limit,
            100_000,
            500,
            Some("user1".to_string()),
        );
        
        order_book.add_order(sell_order).unwrap();
        
        // add a FOK order for more than available
        let fok_order = Order::new(
            Side::Buy,
            OrderType::FillOrKill,
            100_000,
            1000, // more than available (500)
            Some("user2".to_string()),
        );
        
        let trades = order_book.add_order(fok_order).unwrap();
        assert_eq!(trades.len(), 0); // should be rejected
        
        // check that original order is still there
        let snapshot = order_book.get_order_book_snapshot(5);
        assert_eq!(snapshot.asks.len(), 1);
        assert_eq!(snapshot.asks[0].quantity, 500);
    }

    #[test]
    fn test_immediate_or_cancel_order() {
        let mut order_book = OrderBook::new();
        
        // add a sell order
        let sell_order = Order::new(
            Side::Sell,
            OrderType::Limit,
            100_000,
            500,
            Some("user1".to_string()),
        );
        
        order_book.add_order(sell_order).unwrap();
        
        // add an IOC order for more than available
        let ioc_order = Order::new(
            Side::Buy,
            OrderType::ImmediateOrCancel,
            100_000,
            1000, // more than available (500)
            Some("user2".to_string()),
        );
        
        let trades = order_book.add_order(ioc_order).unwrap();
        assert_eq!(trades.len(), 1); // should partially fill
        assert_eq!(trades[0].quantity, 500); // only what's available
        
        // check that original order is gone
        let snapshot = order_book.get_order_book_snapshot(5);
        assert_eq!(snapshot.asks.len(), 0);
    }

    #[test]
    fn test_best_bid_ask() {
        let mut order_book = OrderBook::new();
        
        // add multiple orders
        let buy_order1 = Order::new(Side::Buy, OrderType::Limit, 100_000, 1000, None);
        let buy_order2 = Order::new(Side::Buy, OrderType::Limit, 99_000, 1000, None);
        let sell_order1 = Order::new(Side::Sell, OrderType::Limit, 101_000, 1000, None);
        let sell_order2 = Order::new(Side::Sell, OrderType::Limit, 102_000, 1000, None);
        
        order_book.add_order(buy_order1).unwrap();
        order_book.add_order(buy_order2).unwrap();
        order_book.add_order(sell_order1).unwrap();
        order_book.add_order(sell_order2).unwrap();
        
        assert_eq!(order_book.get_best_bid(), Some(100_000));
        assert_eq!(order_book.get_best_ask(), Some(101_000));
    }

    #[test]
    fn test_order_modification() {
        let mut order_book = OrderBook::new();
        
        let order = Order::new(
            Side::Buy,
            OrderType::Limit,
            100_000,
            1000,
            Some("user1".to_string()),
        );
        
        let order_id = order.id;
        order_book.add_order(order).unwrap();
        
        // modify the order
        let trades = order_book.modify_order(order_id, Some(99_000), Some(500)).unwrap();
        assert_eq!(trades.len(), 0);
        
        let snapshot = order_book.get_order_book_snapshot(5);
        assert_eq!(snapshot.bids.len(), 1);
        assert_eq!(snapshot.bids[0].price, 99_000);
        assert_eq!(snapshot.bids[0].quantity, 500);
    }

    #[tokio::test]
    async fn test_market_data_processing() {
        let order_book = Arc::new(Mutex::new(OrderBook::new()));
        let mut processor = MarketDataProcessor::new(order_book.clone());
        
        let message = MarketDataMessage::NewOrder(NewOrderMessage {
            message_type: MessageType::NewOrder,
            order_id: Uuid::new_v4(),
            side: Side::Buy,
            order_type: OrderType::Limit,
            price: 100_000,
            quantity: 1000,
            timestamp: chrono::Utc::now(),
            sequence_number: 1,
        });
        
        let result = processor.process_market_data(message).await;
        assert!(result.is_ok());
        
        let stats = processor.get_stats();
        assert_eq!(stats.messages_processed, 1);
        assert_eq!(stats.new_orders, 1);
    }
}
