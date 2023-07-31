#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::contract]
pub trait LimitOrderContract {
    #[init]
    fn init(&self) {}
    #[proxy]
    fn contract_proxy(&self, sc_address: ManagedAddress) -> self::Proxy<Self::Api>;

    #[endpoint]
    #[payable("*")]
    fn create_order(&self, price: EgldOrEsdtTokenPayment) -> usize {
        let caller: ManagedAddress = self.blockchain().get_caller();
        let payment: EgldOrEsdtTokenPayment = self.call_value().egld_or_single_esdt();
        self.orders().push(&Order {
            owner: caller,
            payment,
            price,
            active: true,
        });
        self.orders().len()
    }

    #[endpoint]
    fn cancel_order(&self, order_id: usize) {
        let caller: ManagedAddress = self.blockchain().get_caller();
        let mut order: Order<Self::Api> = self.orders().get(order_id);
        require!(order.owner == caller, "Only the owner can cancel the order");
        require!(order.active, "The order is already cancelled");
        order.active = false;
        self.orders().set(order_id, &order);
        self.send().direct(
            &caller,
            &order.payment.token_identifier,
            order.payment.token_nonce,
            &order.payment.amount,
        );
    }

    #[endpoint]
    #[payable("*")]
    fn fill_order(&self, order_id: usize) {
        let mut order: Order<Self::Api> = self.orders().get(order_id);
        require!(order.active, "The order has been cancelled");
        let paid = self.call_value().egld_or_single_esdt();
        require!(
            paid.token_identifier == order.price.token_identifier
                && paid.token_nonce == order.price.token_nonce,
            "The order is not for the same token"
        );
        require!(
            paid.amount >= order.price.amount,
            "The amount is not enough to fill the order"
        );
        let caller = self.blockchain().get_caller();
        self.send().direct(
            &caller,
            &order.payment.token_identifier,
            order.payment.token_nonce,
            &order.payment.amount,
        );
        self.send().direct(
            &order.owner,
            &order.price.token_identifier,
            order.price.token_nonce,
            &order.price.amount,
        );
        order.active = false;
        self.orders().set(order_id, &order);
    }

    #[callback]
    fn after_remote_fill_order(
        &self,
        order_id: usize,
        caller: ManagedAddress,
        remaining: BigUint,
        #[call_result] result: ManagedAsyncCallResult<()>,
    ) {
        require!(false, "error");

        let mut order: Order<Self::Api> = self.orders().get(order_id);
        let payment = self.call_value().egld_or_single_esdt();

        require!(result.is_ok(), "The remote call failed");
        require!(
            payment.token_identifier == order.price.token_identifier
                && payment.token_nonce == order.price.token_nonce,
            "The order is not for the same token"
        );
        require!(
            payment.amount >= order.price.amount,
            "The amount is not enough to fill the order"
        );

        if remaining > 0 {
            self.send().direct(
                &caller,
                &order.payment.token_identifier,
                order.payment.token_nonce,
                &remaining,
            );
        }

        // choice: a limit order can receive more. We declare that the surplus price goes to the owner
        self.send().direct(
            &order.owner,
            &order.price.token_identifier,
            order.price.token_nonce,
            &order.price.amount,
        );
        order.active = false;
        self.orders().set(order_id, &order);
    }

    #[endpoint]
    #[payable("*")]
    fn fill_order_with_other(
        &self,
        order_id: usize,
        other_address: ManagedAddress,
        other_id: usize,
        other_price: EgldOrEsdtTokenPayment,
    ) {
        let order: Order<Self::Api> = self.orders().get(order_id);
        require!(order.active, "The order has been cancelled");
        require!(
            order.payment.token_identifier == other_price.token_identifier
                && order.payment.token_nonce == other_price.token_nonce,
            "The other order is not for the same token"
        );
        let remaining = order.payment.amount - other_price.amount.clone();
        require!(remaining >= 0, "The remaining amount is negative");
        (self
            .contract_proxy(other_address)
            .fill_order(other_id)
            .with_egld_or_single_esdt_transfer((
                other_price.token_identifier,
                other_price.token_nonce,
                other_price.amount,
            ))
            .async_call()
            .with_callback(self.callbacks().after_remote_fill_order(
                order_id,
                self.blockchain().get_caller(),
                remaining,
            ))
            .call_and_exit());
    }

    #[storage_mapper("orders")]
    fn orders(&self) -> VecMapper<Order<Self::Api>>;
}

#[derive(TopEncode, TopDecode, Debug)]
pub struct Order<M: ManagedTypeApi> {
    owner: ManagedAddress<M>,
    payment: EgldOrEsdtTokenPayment<M>,
    price: EgldOrEsdtTokenPayment<M>,
    active: bool,
}
