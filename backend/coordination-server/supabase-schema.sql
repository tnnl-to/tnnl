-- Supabase schema for tnnl coordination server
-- Run this in your Supabase SQL editor

-- Create user_profiles table for storing SSH keys
-- This extends auth.users with tnnl-specific data
CREATE TABLE IF NOT EXISTS public.user_profiles (
    id uuid PRIMARY KEY REFERENCES auth.users(id) ON DELETE CASCADE,
    ssh_public_key text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

-- Create tunnels table
-- Note: user_id references auth.users(id) which already exists in Supabase
CREATE TABLE IF NOT EXISTS public.tunnels (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    subdomain text UNIQUE NOT NULL,
    user_id uuid NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    is_custom boolean NOT NULL DEFAULT false,
    port integer NOT NULL,
    password text, -- Optional HTTP Basic Auth password
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    last_connected_at timestamptz
);

-- Create indexes for faster lookups
CREATE INDEX IF NOT EXISTS idx_tunnels_subdomain ON public.tunnels(subdomain);
CREATE INDEX IF NOT EXISTS idx_tunnels_user_id ON public.tunnels(user_id);
CREATE INDEX IF NOT EXISTS idx_tunnels_port ON public.tunnels(port);

-- Enable Row Level Security (RLS) on both tables
ALTER TABLE public.user_profiles ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.tunnels ENABLE ROW LEVEL SECURITY;

-- RLS Policies for user_profiles
CREATE POLICY "Users can view their own profile"
    ON public.user_profiles
    FOR SELECT
    USING (auth.uid() = id);

CREATE POLICY "Users can update their own profile"
    ON public.user_profiles
    FOR ALL
    USING (auth.uid() = id);

CREATE POLICY "Service role has full access to profiles"
    ON public.user_profiles
    FOR ALL
    TO service_role
    USING (true)
    WITH CHECK (true);

-- RLS Policies: Users can only see/modify their own tunnels
CREATE POLICY "Users can view their own tunnels"
    ON public.tunnels
    FOR SELECT
    USING (auth.uid() = user_id);

CREATE POLICY "Users can create their own tunnels"
    ON public.tunnels
    FOR INSERT
    WITH CHECK (auth.uid() = user_id);

CREATE POLICY "Users can update their own tunnels"
    ON public.tunnels
    FOR UPDATE
    USING (auth.uid() = user_id);

CREATE POLICY "Users can delete their own tunnels"
    ON public.tunnels
    FOR DELETE
    USING (auth.uid() = user_id);

-- Service role can do everything (bypass RLS)
CREATE POLICY "Service role has full access"
    ON public.tunnels
    FOR ALL
    TO service_role
    USING (true)
    WITH CHECK (true);

-- Create updated_at trigger
CREATE OR REPLACE FUNCTION public.handle_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER set_updated_at
    BEFORE UPDATE ON public.tunnels
    FOR EACH ROW
    EXECUTE FUNCTION public.handle_updated_at();

-- Grant permissions
GRANT ALL ON public.user_profiles TO service_role;
GRANT ALL ON public.tunnels TO service_role;
GRANT SELECT, INSERT, UPDATE, DELETE ON public.user_profiles TO authenticated;
GRANT SELECT, INSERT, UPDATE, DELETE ON public.tunnels TO authenticated;
