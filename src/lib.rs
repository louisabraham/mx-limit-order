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
        let caller = self.blockchain().get_caller();
        let address_self = self.blockchain().get_sc_address();

        // prevent flaw: if the transactions are done between shards,
        // the storage will be changed even if they fail
        let shard_self = self.blockchain().get_shard_of_address(&address_self);
        let shard_owner = self
            .blockchain()
            .get_shard_of_address(&self.blockchain().get_owner_address());
        let shard_caller = self.blockchain().get_shard_of_address(&caller);
        require!(
            shard_self == shard_owner && shard_self == shard_caller,
            "The caller or the owner is not on the same shard as the contract"
        );

        if caller != address_self {
            require!(
                paid.token_identifier == order.price.token_identifier
                    && paid.token_nonce == order.price.token_nonce,
                "The order is not for the same token"
            );
            require!(
                paid.amount >= order.price.amount,
                "The amount is not enough to fill the order"
            );
            self.send().direct(
                &caller,
                &order.payment.token_identifier,
                order.payment.token_nonce,
                &order.payment.amount,
            );
        }

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
        let mut order: Order<Self::Api> = self.orders().get(order_id);
        require!(order.active, "The order has been cancelled");
        require!(
            order.payment.token_identifier == other_price.token_identifier
                && order.payment.token_nonce == other_price.token_nonce,
            "The other order is not for the same token"
        );
        let diff_payment = order.payment.amount.clone() - other_price.amount.clone();
        require!(diff_payment >= 0, "The remaining amount is negative");

        let address_self = self.blockchain().get_sc_address();
        let received_price: BigUint;
        if address_self == other_address {
            self.contract_proxy(other_address)
                .fill_order(other_id)
                .execute_on_dest_context::<IgnoreValue>();
            let other_order = self.orders().get(other_id);
            received_price = other_order.payment.amount.clone();
        } else {
            let balance_price_before = self
                .blockchain()
                .get_sc_balance(&order.price.token_identifier, order.price.token_nonce);
            self.contract_proxy(other_address)
                .fill_order(other_id)
                .with_egld_or_single_esdt_transfer((
                    other_price.token_identifier,
                    other_price.token_nonce,
                    other_price.amount,
                ))
                .execute_on_dest_context::<IgnoreValue>();
            let balance_price_after = self
                .blockchain()
                .get_sc_balance(&order.price.token_identifier, order.price.token_nonce);
            received_price = balance_price_after.clone() - balance_price_before.clone();
        }

        require!(
            received_price >= order.price.amount,
            "The other order did not pay enough"
        );

        if diff_payment > 0 {
            self.send().direct(
                &self.blockchain().get_caller(),
                &order.payment.token_identifier,
                order.payment.token_nonce,
                &diff_payment,
            );
        }

        // choice: a limit order can receive more. We declare that the surplus price goes to the owner
        // the surplus price does not exist in practice as the arbitrageur can create another proxy contract
        self.send().direct(
            &order.owner,
            &order.price.token_identifier,
            order.price.token_nonce,
            &received_price,
        );

        order.active = false;
        self.orders().set(order_id, &order);
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
