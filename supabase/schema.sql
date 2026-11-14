-- =====================================================================
-- C02 Naval : schéma Supabase (comptes, statistiques, matchmaking)
-- À coller UNE FOIS dans Supabase : SQL Editor > New query > Run.
-- Peut être relancé sans danger (idempotent).
-- =====================================================================

-- ---------- Liste d'emojis autorisés (identique à EMOJIS dans index.html, tous PEGI 12) ----------
create or replace function public.c02_emojis() returns text[]
language sql immutable as $$ select array['⚓', '🚢', '⛵', '🛥️', '🐙', '🦈', '🐬', '🐳', '🐋', '🐟', '🐠', '🐡', '🦀', '🦞', '🐢', '🐧', '🦅', '🦉', '🦜', '🐺', '🦊', '🐻', '🐼', '🦁', '🐯', '🐸', '🐵', '🦄', '🐲', '🌊', '🏝️', '🌴', '☀️', '🌙', '⭐', '🚀', '🛸', '🔥', '⚡', '🌈', '🍀', '🎯', '🏆', '👑', '🎮', '🎲', '🧭', '🔭', '🤖', '👾', '😀', '😎', '🤠', '🥳', '😺']::text[] $$;

-- ---------- Profils ----------
create table if not exists public.profiles (
  id uuid primary key references auth.users(id) on delete cascade,
  name text not null,
  emoji text not null,
  xp integer not null default 0,
  games integer not null default 0,
  wins integer not null default 0,
  losses integer not null default 0,
  interrupted integer not null default 0,
  streak integer not null default 0,
  best_streak integer not null default 0,
  shots integer not null default 0,
  hits integer not null default 0,
  sunk integer not null default 0,
  upgrades integer not null default 0,
  fastest_win integer,
  time_played integer not null default 0,
  imported boolean not null default false,
  last_game_at timestamptz,
  created_at timestamptz not null default now()
);
create unique index if not exists profiles_name_lower_idx on public.profiles (lower(name));

alter table public.profiles enable row level security;
drop policy if exists "profil : lecture de son propre profil" on public.profiles;
create policy "profil : lecture de son propre profil" on public.profiles
  for select to authenticated using (id = auth.uid());
-- Aucune politique insert / update / delete : toute écriture passe par les fonctions ci-dessous,
-- qui valident les données (un joueur ne peut donc pas s'attribuer l'XP qu'il veut).

-- ---------- File d'attente du matchmaking ----------
create table if not exists public.queue (
  ticket text primary key,
  room text not null,
  created_at timestamptz not null default now()
);
alter table public.queue enable row level security;   -- aucune politique : accessible uniquement via matchmake()

-- ---------- Création du profil ----------
create or replace function public.create_profile(p_name text, p_emoji text)
returns public.profiles language plpgsql security definer set search_path = public as $$
declare
  v_name text := regexp_replace(btrim(coalesce(p_name, '')), '\s+', ' ', 'g');
  r public.profiles;
begin
  if auth.uid() is null then raise exception 'not_authenticated'; end if;
  if char_length(v_name) < 2 or char_length(v_name) > 16
     or v_name ~ '[\u0001-\u001F\u200B-\u200F\u202A-\u202E\u2060-\u206F\uFEFF]' then
    raise exception 'name_invalid';
  end if;
  if p_emoji is null or not (p_emoji = any (public.c02_emojis())) then raise exception 'emoji_invalid'; end if;
  select * into r from public.profiles where id = auth.uid();
  if found then return r; end if;                       -- déjà créé (double clic, deux onglets…)
  begin
    insert into public.profiles (id, name, emoji) values (auth.uid(), v_name, p_emoji) returning * into r;
  exception when unique_violation then
    raise exception 'name_taken';
  end;
  return r;
end $$;

-- ---------- Modification du pseudo et de l'emoji ----------
create or replace function public.update_profile(p_name text, p_emoji text)
returns public.profiles language plpgsql security definer set search_path = public as $$
declare
  v_name text := regexp_replace(btrim(coalesce(p_name, '')), '\s+', ' ', 'g');
  r public.profiles;
begin
  if auth.uid() is null then raise exception 'not_authenticated'; end if;
  if char_length(v_name) < 2 or char_length(v_name) > 16
     or v_name ~ '[\u0001-\u001F\u200B-\u200F\u202A-\u202E\u2060-\u206F\uFEFF]' then
    raise exception 'name_invalid';
  end if;
  if p_emoji is null or not (p_emoji = any (public.c02_emojis())) then raise exception 'emoji_invalid'; end if;
  begin
    update public.profiles set name = v_name, emoji = p_emoji where id = auth.uid() returning * into r;
  exception when unique_violation then
    raise exception 'name_taken';
  end;
  if not found then raise exception 'no_profile'; end if;
  return r;
end $$;

-- ---------- Enregistrement d'une partie (l'XP est calculée ICI, jamais envoyée par le client) ----------
create or replace function public.record_game(
  p_win boolean default false, p_shots integer default 0, p_hits integer default 0, p_sunk integer default 0,
  p_upgrades integer default 0, p_seconds integer default 0, p_halved boolean default false, p_interrupted boolean default false)
returns jsonb language plpgsql security definer set search_path = public as $$
declare
  r public.profiles;
  v_win boolean := coalesce(p_win, false);
  v_shots integer; v_hits integer; v_sunk integer; v_up integer; v_sec integer; v_gain integer; v_acc numeric;
begin
  if auth.uid() is null then raise exception 'not_authenticated'; end if;
  select * into r from public.profiles where id = auth.uid() for update;
  if not found then raise exception 'no_profile'; end if;
  v_sec := least(greatest(coalesce(p_seconds, 0), 0), 7200);

  if coalesce(p_interrupted, false) then
    update public.profiles set interrupted = interrupted + 1, time_played = time_played + v_sec
      where id = r.id returning * into r;
    return jsonb_build_object('gain', 0, 'profile', to_jsonb(r));
  end if;

  -- anti-abus : une partie comptabilisée toutes les 20 secondes au plus
  if r.last_game_at is not null and now() - r.last_game_at < interval '20 seconds' then
    return jsonb_build_object('gain', 0, 'limited', true, 'profile', to_jsonb(r));
  end if;

  v_shots := least(greatest(coalesce(p_shots, 0), 0), 400);
  v_hits  := least(greatest(coalesce(p_hits, 0), 0), v_shots);
  v_sunk  := least(greatest(coalesce(p_sunk, 0), 0), 10);
  v_up    := least(greatest(coalesce(p_upgrades, 0), 0), 20);
  v_acc   := case when v_shots > 0 then v_hits::numeric / v_shots else 0 end;
  v_gain  := (case when v_win then 100 else 30 end) + v_hits * 5 + v_sunk * 20 + (case when v_win then round(v_acc * 50)::integer else 0 end);
  if coalesce(p_halved, false) then v_gain := round(v_gain / 2.0)::integer; end if;
  v_gain := least(v_gain, 400);

  update public.profiles set
    xp = xp + v_gain,
    games = games + 1,
    wins = wins + (case when v_win then 1 else 0 end),
    losses = losses + (case when v_win then 0 else 1 end),
    streak = case when v_win then streak + 1 else 0 end,
    best_streak = greatest(best_streak, case when v_win then streak + 1 else 0 end),
    shots = shots + v_shots, hits = hits + v_hits, sunk = sunk + v_sunk, upgrades = upgrades + v_up,
    fastest_win = case when v_win and v_shots > 0 and (fastest_win is null or v_shots < fastest_win) then v_shots else fastest_win end,
    time_played = time_played + v_sec,
    last_game_at = now()
  where id = r.id returning * into r;
  return jsonb_build_object('gain', v_gain, 'profile', to_jsonb(r));
end $$;

-- ---------- Import unique des statistiques d'un ancien compte local (valeurs plafonnées) ----------
create or replace function public.import_local(p_stats jsonb)
returns public.profiles language plpgsql security definer set search_path = public as $$
declare
  r public.profiles;
  g integer; w integer; l integer;
begin
  if auth.uid() is null then raise exception 'not_authenticated'; end if;
  select * into r from public.profiles where id = auth.uid() for update;
  if not found then raise exception 'no_profile'; end if;
  if r.imported or r.games > 0 then raise exception 'already_imported'; end if;
  g := least(greatest(coalesce((p_stats->>'games')::integer, 0), 0), 500);
  w := least(greatest(coalesce((p_stats->>'wins')::integer, 0), 0), g);
  l := least(greatest(coalesce((p_stats->>'losses')::integer, 0), 0), g - w);
  update public.profiles set
    xp = least(greatest(coalesce((p_stats->>'xp')::integer, 0), 0), 5000),
    games = g, wins = w, losses = l,
    interrupted = least(greatest(coalesce((p_stats->>'interrupted')::integer, 0), 0), 500),
    best_streak = least(greatest(coalesce((p_stats->>'bestStreak')::integer, 0), 0), w),
    shots = least(greatest(coalesce((p_stats->>'shots')::integer, 0), 0), 50000),
    hits = least(greatest(coalesce((p_stats->>'hits')::integer, 0), 0), least(greatest(coalesce((p_stats->>'shots')::integer, 0), 0), 50000)),
    sunk = least(greatest(coalesce((p_stats->>'sunk')::integer, 0), 0), 5000),
    upgrades = least(greatest(coalesce((p_stats->>'upgrades')::integer, 0), 0), 5000),
    fastest_win = nullif(least(greatest(coalesce((p_stats->>'fastestWin')::integer, 0), 0), 400), 0),
    time_played = least(greatest(coalesce((p_stats->>'timePlayed')::integer, 0), 0), 3600000),
    imported = true
  where id = r.id returning * into r;
  return r;
end $$;

-- ---------- Suppression du compte (profil et identifiant de connexion) ----------
create or replace function public.delete_account()
returns void language plpgsql security definer set search_path = public as $$
begin
  if auth.uid() is null then raise exception 'not_authenticated'; end if;
  delete from auth.users where id = auth.uid();       -- le profil est supprimé en cascade
end $$;

-- ---------- Matchmaking : renvoie le salon d'un joueur en attente, ou t'en crée un ----------
create or replace function public.matchmake(p_ticket text, p_room text)
returns jsonb language plpgsql security definer set search_path = public as $$
declare v_room text;
begin
  if p_ticket is null or char_length(p_ticket) not between 6 and 40 or p_room is null or char_length(p_room) <> 6 then
    raise exception 'bad_request';
  end if;
  delete from public.queue where created_at < now() - interval '60 seconds';
  delete from public.queue where ticket = p_ticket;
  with picked as (select ticket from public.queue order by created_at limit 1 for update skip locked)
  delete from public.queue q using picked where q.ticket = picked.ticket returning q.room into v_room;
  if v_room is not null then return jsonb_build_object('role', 'guest', 'room', v_room); end if;
  insert into public.queue (ticket, room) values (p_ticket, p_room);
  return jsonb_build_object('role', 'host', 'room', p_room);
end $$;

create or replace function public.leave_queue(p_ticket text)
returns void language sql security definer set search_path = public as $$
  delete from public.queue where ticket = p_ticket
$$;

-- ---------- Droits d'exécution ----------
revoke all on function public.create_profile(text, text), public.update_profile(text, text),
  public.record_game(boolean, integer, integer, integer, integer, integer, boolean, boolean),
  public.import_local(jsonb), public.delete_account(), public.matchmake(text, text), public.leave_queue(text) from public;
grant execute on function public.create_profile(text, text), public.update_profile(text, text),
  public.record_game(boolean, integer, integer, integer, integer, integer, boolean, boolean),
  public.import_local(jsonb), public.delete_account() to authenticated;
grant execute on function public.matchmake(text, text), public.leave_queue(text) to anon, authenticated;
grant select on public.profiles to authenticated;

-- Supabase accorde par défaut les fonctions à « anon » : on le retire pour tout ce qui exige un compte.
revoke execute on function public.create_profile(text, text), public.update_profile(text, text),
  public.record_game(boolean, integer, integer, integer, integer, integer, boolean, boolean),
  public.import_local(jsonb), public.delete_account() from anon;
alter function public.c02_emojis() set search_path = public;
