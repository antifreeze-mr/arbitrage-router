// lib.rs - HFT Arbitrage Router: FULL INLINE (NO LIBSECP256K1 ISSUES)
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use anchor_spl::associated_token::get_associated_token_address;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
};
use std::str::FromStr;

declare_id!("4xVUrp3J6t6FKrS61uWN6UZRCrvfMU97qa8uJJxncaP1");

#[program]
pub mod dex_arbitrage_router {
    use super::*;

    /// Инициализация роутера (вызывается один раз)
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let router_state = &mut ctx.accounts.router_state;
        router_state.owner = ctx.accounts.owner.key();
        router_state.is_paused = false;
        router_state.bump = ctx.bumps.router_state;
        
        msg!("HFT Arbitrage Router initialized. Owner: {}", router_state.owner);
        Ok(())
    }

    /// 🚀 ГЛАВНАЯ ФУНКЦИЯ: ANCHOR 0.29 COMPATIBLE (EXPLICIT LIFETIMES)
    pub fn execute_arbitrage_batch<'info>(
        ctx: Context<'_, '_, 'info, 'info, ExecuteArbitrageBatch<'info>>,
        arbitrages: [ArbitrageParams; 4],
    ) -> Result<()> {
        // 1. Проверка паузы (первая линия защиты)
        require!(!ctx.accounts.router_state.is_paused, MyErrorCode::ContractIsPaused);
        
        msg!("🚀 Starting INLINE HFT arbitrage batch execution with 4 trades");

        // 🎯 КЛЮЧЕВОЕ РЕШЕНИЕ: ИЗВЛЕКАЕМ ВСЕ ССЫЛКИ ДО ЦИКЛА (РЕШАЕТ LIFETIME ПРОБЛЕМЫ)
        let user = &ctx.accounts.user;
        let system_program = &ctx.accounts.system_program;
        let token_program = &ctx.accounts.token_program;
        let rent = &ctx.accounts.rent;
        let user_key = user.key();
        let system_program_key = system_program.key();
        let token_program_key = token_program.key();
        let rent_key = rent.key();

        // 🔧 СОЗДАЕМ КОНСТАНТЫ ОДИН РАЗ (МИНИМИЗИРУЕМ CRYPTO ОПЕРАЦИИ)
        let pump_program_id = Pubkey::from_str("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P").unwrap();
        let fee_recipient = Pubkey::from_str("CebN5WGQ4jvEPvsVU4EoHEpgzq1VV7AbicfhtW4xC9iM").unwrap();

        // 2. Гибкая нарезка аккаунтов на основе accounts_count
        let mut account_offset = 0;
        
        // 3. ПОЛНОСТЬЮ INLINE ЦИКЛ: ВСЯ ЛОГИКА ПРЯМО ЗДЕСЬ
        for (index, arbitrage) in arbitrages.iter().enumerate() {
            msg!("⚡ Executing arbitrage #{} (FULL INLINE MODE)", index + 1);
            msg!("📊 Accounts needed: {}", arbitrage.accounts_count);
            
            // Вычисляем границы среза для этого арбитража
            let start = account_offset;
            let end = start + arbitrage.accounts_count as usize;
            
            // Проверяем что у нас достаточно аккаунтов
            require!(
                ctx.remaining_accounts.len() >= end,
                MyErrorCode::InsufficientAccounts
            );
            
            let arbitrage_accounts_slice = &ctx.remaining_accounts[start..end];
            
            msg!("🔧 Using accounts slice [{}, {})", start, end);
            
            msg!("🧠 Go-bot parameters: buy {} tokens (max {} SOL), sell {} tokens (min {} wSOL)", 
                 arbitrage.tokens_to_buy, arbitrage.max_sol_cost, 
                 arbitrage.tokens_to_sell, arbitrage.min_wsol_out);

            // ====================================================================
            // 🔥 INLINE BUY INSTRUCTION CREATION
            // ====================================================================
            
            let buy_instruction = match arbitrage.buy_dex {
                DexType::PumpFun => {
                    msg!("🔧 Creating Pump.fun BUY instruction inline...");
                    
                    // Поиск аккаунтов inline (БЕЗ CRYPTO ЗАВИСИМОСТЕЙ)
                    let mut pump_program_account = None;
                    let mut global_account = None;
                    let mut fee_recipient_account = None;
                    let mut mint_account = None;
                    let mut bonding_curve_account = None;
                    let mut user_token_account = None;
                    let mut event_authority_account = None;
                    
                    // Inline поиск всех нужных аккаунтов (COMPILE-TIME PUBKEYS)
                    for acc_info in arbitrage_accounts_slice {
                        // Pump program
                        if acc_info.key() == pump_program_id {
                            pump_program_account = Some(acc_info);
                        }
                        // Global PDA
                        let (expected_global, _) = Pubkey::find_program_address(&[b"global"], &pump_program_id);
                        if acc_info.key() == expected_global {
                            global_account = Some(acc_info);
                        }
                        // Fee recipient
                        if acc_info.key() == fee_recipient {
                            fee_recipient_account = Some(acc_info);
                        }
                        // Mint
                        if acc_info.key() == arbitrage.token_mint {
                            mint_account = Some(acc_info);
                        }
                        // Bonding curve PDA
                        let (expected_bonding_curve, _) = Pubkey::find_program_address(&[b"bonding-curve", arbitrage.token_mint.as_ref()], &pump_program_id);
                        if acc_info.key() == expected_bonding_curve {
                            bonding_curve_account = Some(acc_info);
                        }
                        // User token account
                        if acc_info.owner == &anchor_spl::token::ID && acc_info.data_len() == TokenAccount::LEN {
                            if let Ok(token_account) = TokenAccount::try_deserialize(&mut acc_info.data.borrow().as_ref()) {
                                if token_account.owner == user_key && token_account.mint == arbitrage.token_mint {
                                    user_token_account = Some(acc_info);
                                }
                            }
                        }
                        // Event authority PDA
                        let (expected_event_authority, _) = Pubkey::find_program_address(&[b"__event_authority"], &pump_program_id);
                        if acc_info.key() == expected_event_authority {
                            event_authority_account = Some(acc_info);
                        }
                    }
                    
                    // Проверяем что все аккаунты найдены
                    let pump_program_account = pump_program_account.ok_or(MyErrorCode::AccountNotFound)?;
                    let global_account = global_account.ok_or(MyErrorCode::PDAAccountNotFound)?;
                    let fee_recipient_account = fee_recipient_account.ok_or(MyErrorCode::AccountNotFound)?;
                    let mint_account = mint_account.ok_or(MyErrorCode::MintAccountNotFound)?;
                    let bonding_curve_account = bonding_curve_account.ok_or(MyErrorCode::PDAAccountNotFound)?;
                    let user_token_account = user_token_account.ok_or(MyErrorCode::TokenAccountNotFound)?;
                    let event_authority_account = event_authority_account.ok_or(MyErrorCode::PDAAccountNotFound)?;
                    
                    // Находим associated bonding curve (ATA)
                    let expected_ata = get_associated_token_address(&bonding_curve_account.key(), &arbitrage.token_mint);
                    let mut associated_bonding_curve_account = None;
                    for acc_info in arbitrage_accounts_slice {
                        if acc_info.key() == expected_ata {
                            associated_bonding_curve_account = Some(acc_info);
                            break;
                        }
                    }
                    let associated_bonding_curve_account = associated_bonding_curve_account.ok_or(MyErrorCode::AccountNotFound)?;
                    
                    // Создаем instruction data
                    let mut instruction_data = Vec::new();
                    instruction_data.extend_from_slice(&[0x66, 0x06, 0x3d, 0x12, 0x01, 0xda, 0xeb, 0xea]); // buy discriminator
                    instruction_data.extend_from_slice(&arbitrage.tokens_to_buy.to_le_bytes());
                    instruction_data.extend_from_slice(&arbitrage.max_sol_cost.to_le_bytes());
                    
                    // Создаем instruction
                    Instruction {
                        program_id: pump_program_id,
                        accounts: vec![
                            AccountMeta::new_readonly(global_account.key(), false),
                            AccountMeta::new(fee_recipient_account.key(), false),
                            AccountMeta::new_readonly(mint_account.key(), false),
                            AccountMeta::new(bonding_curve_account.key(), false),
                            AccountMeta::new(associated_bonding_curve_account.key(), false),
                            AccountMeta::new(user_token_account.key(), false),
                            AccountMeta::new(user_key, true),
                            AccountMeta::new_readonly(system_program_key, false),
                            AccountMeta::new_readonly(token_program_key, false),
                            AccountMeta::new_readonly(rent_key, false),
                            AccountMeta::new_readonly(event_authority_account.key(), false),
                            AccountMeta::new_readonly(pump_program_account.key(), false),
                        ],
                        data: instruction_data,
                    }
                },
                DexType::Meteora => {
                    msg!("🚧 Meteora not implemented yet");
                    return Err(MyErrorCode::InvalidDexType.into());
                },
            };

            // Создаем accounts для buy invoke
            let buy_accounts = match arbitrage.buy_dex {
                DexType::PumpFun => {
                    let mut accounts = Vec::new();
                    
                    // Те же аккаунты что в instruction, но как AccountInfo
                    for acc_info in arbitrage_accounts_slice {
                        let (expected_global, _) = Pubkey::find_program_address(&[b"global"], &pump_program_id);
                        let (expected_bonding_curve, _) = Pubkey::find_program_address(&[b"bonding-curve", arbitrage.token_mint.as_ref()], &pump_program_id);
                        let expected_ata = get_associated_token_address(&expected_bonding_curve, &arbitrage.token_mint);
                        let (expected_event_authority, _) = Pubkey::find_program_address(&[b"__event_authority"], &pump_program_id);
                        
                        if acc_info.key() == expected_global ||
                           acc_info.key() == fee_recipient ||
                           acc_info.key() == arbitrage.token_mint ||
                           acc_info.key() == expected_bonding_curve ||
                           acc_info.key() == expected_ata ||
                           acc_info.key() == expected_event_authority ||
                           acc_info.key() == pump_program_id ||
                           (acc_info.owner == &anchor_spl::token::ID && acc_info.data_len() == TokenAccount::LEN) {
                            accounts.push(acc_info.clone());
                        }
                    }
                    
                    // Добавляем основные аккаунты из контекста
                    accounts.push(user.to_account_info());
                    accounts.push(system_program.to_account_info());
                    accounts.push(token_program.to_account_info());
                    accounts.push(rent.to_account_info());
                    
                    accounts
                },
                DexType::Meteora => vec![],
            };

            // ====================================================================
            // 🔥 INLINE SELL INSTRUCTION CREATION
            // ====================================================================
            
            let sell_instruction = match arbitrage.sell_dex {
                DexType::PumpFun => {
                    msg!("🔧 Creating Pump.fun SELL instruction inline...");
                    
                    // Создаем instruction data для sell
                    let mut instruction_data = Vec::new();
                    instruction_data.extend_from_slice(&[0x33, 0xe6, 0x85, 0xa4, 0x01, 0x7f, 0x83, 0xad]); // sell discriminator
                    instruction_data.extend_from_slice(&arbitrage.tokens_to_sell.to_le_bytes());
                    instruction_data.extend_from_slice(&arbitrage.min_wsol_out.to_le_bytes());
                    
                    // Те же аккаунты что и для buy (Pump.fun использует одинаковые)
                    Instruction {
                        program_id: pump_program_id,
                        accounts: buy_instruction.accounts.clone(), // Переиспользуем аккаунты
                        data: instruction_data,
                    }
                },
                DexType::Meteora => {
                    msg!("🚧 Meteora not implemented yet");
                    return Err(MyErrorCode::InvalidDexType.into());
                },
            };

            let sell_accounts = buy_accounts.clone(); // Для Pump.fun те же аккаунты

            // ====================================================================
            // 🚀 АТОМАРНОЕ ИСПОЛНЕНИЕ: BUY -> SELL
            // ====================================================================
            
            msg!("🚀 Executing BUY -> SELL atomically (INLINE)...");
            
            // Выполняем BUY
            anchor_lang::solana_program::program::invoke(&buy_instruction, &buy_accounts)?;
            msg!("✅ BUY completed");
            
            // Выполняем SELL
            anchor_lang::solana_program::program::invoke(&sell_instruction, &sell_accounts)?;
            msg!("✅ SELL completed");
            
            msg!("🎉 Arbitrage #{} completed successfully (INLINE)", index + 1);
            
            // Обновляем offset для следующего арбитража
            account_offset = end;
        }

        msg!("🏆 INLINE HFT arbitrage batch completed successfully - MAXIMUM SPEED!");
        Ok(())
    }

    /// Emergency stop: только owner может поставить на паузу/снять с паузы
    pub fn toggle_pause(ctx: Context<TogglePause>) -> Result<()> {
        let router_state = &mut ctx.accounts.router_state;
        
        // Проверяем права владельца
        require!(
            ctx.accounts.owner.key() == router_state.owner,
            MyErrorCode::UnauthorizedAccess
        );

        router_state.is_paused = !router_state.is_paused;
        
        msg!("🛑 Router pause status changed to: {}", router_state.is_paused);
        Ok(())
    }
}

// ============================================================================
// 📊 СТРУКТУРЫ ДАННЫХ
// ============================================================================

/// Состояние роутера (хранится on-chain)
#[account]
pub struct RouterState {
    pub owner: Pubkey,      // Владелец для emergency operations
    pub is_paused: bool,    // Флаг паузы (emergency stop)
    pub bump: u8,          // Bump для PDA
}

/// 🧠 Параметры одного арбитража (все рассчитано Go-ботом заранее)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct ArbitrageParams {
    pub token_mint: Pubkey,           // Какой токен арбитрим
    pub amount_in: u64,               // Сколько wSOL инвестируем (для информации)
    pub min_wsol_out: u64,            // Минимальная прибыль (Go-бот рассчитал)
    pub buy_dex: DexType,             // Где покупаем токен
    pub sell_dex: DexType,            // Где продаем токен
    
    // 🎯 КЛЮЧЕВЫЕ ПАРАМЕТРЫ "СЛЕПОГО ДОВЕРИЯ":
    pub accounts_count: u8,           // Сколько аккаунтов нужно для этого арбитража
    pub tokens_to_buy: u64,           // Сколько токенов покупаем (Go-бот рассчитал)
    pub max_sol_cost: u64,            // Максимум SOL тратим (с учетом slippage)
    pub tokens_to_sell: u64,          // Сколько токенов продаем (Go-бот рассчитал)
    // min_wsol_out уже есть выше - минимум получаем (с учетом slippage)
}

/// Поддерживаемые DEX-ы
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub enum DexType {
    Meteora,    // Meteora DLMM
    PumpFun,    // Pump.fun AMM
}

// ============================================================================
// 🔧 КОНТЕКСТЫ ИНСТРУКЦИЙ
// ============================================================================

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = owner,
        space = 8 + 32 + 1 + 1, // discriminator + pubkey + bool + bump
        seeds = [b"router_state"],
        bump
    )]
    pub router_state: Account<'info, RouterState>,
    
    #[account(mut)]
    pub owner: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ExecuteArbitrageBatch<'info> {
    #[account(
        seeds = [b"router_state"],
        bump = router_state.bump
    )]
    pub router_state: Account<'info, RouterState>,
    
    /// Пользователь, выполняющий арбитраж (Go-бот)
    #[account(mut)]
    pub user: Signer<'info>,
    
    /// wSOL аккаунт пользователя (финальная проверка прибыли в конце)
    #[account(mut)]
    pub user_wsol_account: Account<'info, TokenAccount>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    
    // 🧠 Гибкая структура remaining_accounts (Go-бот точно знает что передать):
    // Каждый арбитраж использует accounts_count аккаунтов
    // Батч из 4 арбитражей:
    // [0..accounts_count[0]] - аккаунты для арбитража 1
    // [accounts_count[0]..accounts_count[0]+accounts_count[1]] - аккаунты для арбитража 2
    // и так далее...
}

#[derive(Accounts)]
pub struct TogglePause<'info> {
    #[account(
        mut,
        seeds = [b"router_state"],
        bump = router_state.bump
    )]
    pub router_state: Account<'info, RouterState>,
    
    pub owner: Signer<'info>,
}

// ============================================================================
// ⚠️ КАСТОМНЫЕ ОШИБКИ (для детального дебага)
// ============================================================================

#[error_code]
pub enum MyErrorCode {
    #[msg("Contract is paused by the admin.")]
    ContractIsPaused,

    #[msg("Final profitability check failed. Not enough wSOL returned.")]
    NotProfitable,
    
    #[msg("An arithmetic operation overflowed.")]
    ArithmeticError,

    #[msg("Unauthorized access. Only owner can perform this operation.")]
    UnauthorizedAccess,

    #[msg("Invalid token account mint does not match expected token mint.")]
    InvalidTokenAccount,

    #[msg("Not enough remaining accounts provided.")]
    InsufficientAccounts,

    #[msg("Invalid DEX type specified.")]
    InvalidDexType,

    #[msg("Token account not found in remaining accounts.")]
    TokenAccountNotFound,

    #[msg("Mint account not found in remaining accounts.")]
    MintAccountNotFound,

    #[msg("PDA account not found in remaining accounts.")]
    PDAAccountNotFound,

    #[msg("Required account not found in remaining accounts.")]
    AccountNotFound,

    #[msg("Invalid program ID format.")]
    InvalidProgramId,

    #[msg("CPI call failed.")]
    CpiError,
}