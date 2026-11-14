-- Tests de supabase/schema.sql. Toute assertion fausse fait échouer le script (psql -v ON_ERROR_STOP=1).
-- Lancement : psql -v ON_ERROR_STOP=1 -f supabase/tests/stub.sql -f supabase/schema.sql -f supabase/tests/schema.test.sql
\set A '11111111-1111-1111-1111-111111111111'
\set B '22222222-2222-2222-2222-222222222222'
\o /dev/null
delete from public.profiles; delete from public.queue; delete from auth.users;
insert into auth.users (id, email) values (:'A', 'a@x'), (:'B', 'b@x');

-- Aide : vérifie qu'une instruction échoue avec un message donné.
create or replace function public.t_expect_error(p_sql text, p_msg text) returns void language plpgsql as $$
begin
  begin execute p_sql; exception when others then
    if sqlerrm like '%' || p_msg || '%' then return; end if;
    raise exception 'erreur inattendue pour [%] : % (attendu : %)', p_sql, sqlerrm, p_msg;
  end;
  raise exception 'aucune erreur pour [%] (attendu : %)', p_sql, p_msg;
end $$;
grant execute on function public.t_expect_error(text, text) to anon, authenticated;

-- ===== création et validation du profil =====
set role authenticated; select set_config('request.jwt.claim.sub', :'A', false);
select public.t_expect_error($q$select public.create_profile('A', '🐙')$q$, 'name_invalid');
select public.t_expect_error($q$select public.create_profile('Nom beaucoup trop long', '🐙')$q$, 'name_invalid');
select public.t_expect_error(format('select public.create_profile(%L, %L)', 'Cap' || chr(8203) || 'i', '🐙'), 'name_invalid');
select public.t_expect_error($q$select public.create_profile('Capitaine', '💣')$q$, 'emoji_invalid');
do $$ declare r public.profiles; begin
  r := public.create_profile('  Capitaine  ', '🐙');
  assert r.name = 'Capitaine' and r.emoji = '🐙' and r.xp = 0, 'profil créé et normalisé';
  assert (public.create_profile('Autre', '🦈')).name = 'Capitaine', 'création idempotente';
end $$;
select set_config('request.jwt.claim.sub', :'B', false);
select public.t_expect_error($q$select public.create_profile('CAPITAINE', '🐬')$q$, 'name_taken');   -- insensible à la casse
do $$ begin assert (public.create_profile('Matelot', '🐬')).name = 'Matelot'; end $$;
select public.t_expect_error($q$select public.update_profile('capitaine', '🐬')$q$, 'name_taken');
do $$ begin assert (public.update_profile('Mousse', '🦈')).name = 'Mousse'; end $$;

-- ===== droits : lecture de son seul profil, aucune écriture directe =====
do $$ begin assert (select count(*) from public.profiles) = 1, 'RLS : B ne voit que sa fiche'; end $$;
select public.t_expect_error('update public.profiles set xp = 99999', 'permission denied');
select public.t_expect_error($q$insert into public.profiles (id, name, emoji) values (gen_random_uuid(), 'x1', '🐙')$q$, 'permission denied');
select public.t_expect_error('delete from public.profiles', 'permission denied');
select public.t_expect_error('select * from public.queue', 'permission denied');

-- ===== record_game : XP calculée côté serveur, plafonds, limite de fréquence =====
select set_config('request.jwt.claim.sub', :'A', false);
do $$ declare r jsonb; begin
  r := public.record_game(true, 20, 10, 2, 1, 120, false, false);
  assert (r->>'gain')::int = 215, 'victoire : 100 + 10*5 + 2*20 + 25 = 215';
  assert (r->'profile'->>'wins')::int = 1 and (r->'profile'->>'fastest_win')::int = 20, 'stats victoire';
  r := public.record_game(true, 20, 10, 2, 1, 120, false, false);
  assert (r->>'limited')::boolean and (r->>'gain')::int = 0, 'limite : une partie toutes les 20 s';
end $$;
reset role; update public.profiles set last_game_at = now() - interval '1 minute';
set role authenticated; select set_config('request.jwt.claim.sub', :'A', false);
do $$ declare r jsonb; begin
  r := public.record_game(false, 10, 3, 1, 0, 60, true, false);
  assert (r->>'gain')::int = 33, 'défaite contre le bot : (30 + 15 + 20) / 2 = 33';
  assert (r->'profile'->>'streak')::int = 0 and (r->'profile'->>'losses')::int = 1, 'série remise à zéro';
end $$;
reset role; update public.profiles set last_game_at = now() - interval '1 minute';
set role authenticated; select set_config('request.jwt.claim.sub', :'A', false);
do $$ declare r jsonb; begin
  r := public.record_game(true, 999999, 999999, 999999, 999999, 999999, false, false);
  assert (r->>'gain')::int = 400, 'gain plafonné à 400';
  assert (r->'profile'->>'hits')::int <= (r->'profile'->>'shots')::int, 'touchés <= tirs';
  r := public.record_game(false, 0, 0, 0, 0, 300, false, true);
  assert (r->'profile'->>'interrupted')::int = 1, 'partie interrompue comptée';
end $$;

-- ===== import unique et plafonné =====
select set_config('request.jwt.claim.sub', :'B', false);
do $$ declare r public.profiles; begin
  r := public.import_local('{"xp": 999999, "games": 10, "wins": 40, "losses": 9, "shots": 100, "hits": 500, "bestStreak": 99}'::jsonb);
  assert r.xp = 5000 and r.games = 10 and r.wins = 10 and r.hits = 100 and r.best_streak = 10, 'import plafonné';
end $$;
select public.t_expect_error($q$select public.import_local('{"xp": 5}'::jsonb)$q$, 'already_imported');
select set_config('request.jwt.claim.sub', :'A', false);
select public.t_expect_error($q$select public.import_local('{"xp": 5}'::jsonb)$q$, 'already_imported');   -- A a déjà joué

-- ===== matchmaking (joueurs anonymes) =====
reset role; set role anon; select set_config('request.jwt.claim.sub', '', false);
do $$ declare r jsonb; begin
  r := public.matchmake('ticketAAA', 'ROOM01'); assert r->>'role' = 'host' and r->>'room' = 'ROOM01', 'premier joueur : hôte';
  r := public.matchmake('ticketBBB', 'ROOM02'); assert r->>'role' = 'guest' and r->>'room' = 'ROOM01', 'deuxième joueur : invité du premier';
  r := public.matchmake('ticketCCC', 'ROOM03'); assert r->>'role' = 'host', 'troisième joueur : nouvel hôte';
  perform public.leave_queue('ticketCCC');
end $$;
select public.t_expect_error($q$select public.matchmake('x', 'ROOM05')$q$, 'bad_request');
select public.t_expect_error($q$select public.create_profile('Zed', '🐙')$q$, 'permission denied');
select public.t_expect_error($q$select public.record_game(true)$q$, 'permission denied');
reset role;
do $$ begin assert (select count(*) from public.queue) = 0, 'file vide après appariement et départ'; end $$;
-- entrées périmées nettoyées
insert into public.queue (ticket, room, created_at) values ('vieux-ticket', 'OLD001', now() - interval '5 minutes');
set role anon;
do $$ begin assert public.matchmake('ticketDDD', 'ROOM04')->>'role' = 'host', 'entrée périmée ignorée'; end $$;

-- ===== suppression du compte =====
reset role; set role authenticated; select set_config('request.jwt.claim.sub', :'B', false);
select public.delete_account();
reset role;
do $$ begin assert not exists (select 1 from auth.users where id = '22222222-2222-2222-2222-222222222222') and
  not exists (select 1 from public.profiles where id = '22222222-2222-2222-2222-222222222222'), 'compte et profil supprimés'; end $$;
drop function public.t_expect_error(text, text);
\o
\echo 'TOUS LES TESTS SQL PASSENT'
