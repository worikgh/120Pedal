#!/usr/bin/perl -w
use strict;
print STDERR "LV2_PATH: $ENV{LV2_PATH}\n";
my @commands = ();

## Read the "pedalboard" definitions created by `mod-ui`
## Create a set of files in the pedal directory ($ENV{PEDAL_DIR}):
## * `Initialise`: A file with commands for `mod-host` and `jackd` to
##     set up the simulators needed for all the pedalboards
## * One file for each pedalboard that contains the instructions for
##     jackd to make that pedalboard active

## Delete any existing pedal definitions
my $pedal_dir = $ENV{PEDAL_DIR};
-d $pedal_dir or die "'$pedal_dir' not a directory";
opendir(my $dir, $pedal_dir) or die "$!: $pedal_dir";
foreach my $fn (readdir($dir)){
    $fn =~ /^\./ and next;
    $fn =~ /^[A-Z]$/ and next; # Links for pedal driver
    my $path_to_delete = $pedal_dir .'/'. $fn;
    unlink($path_to_delete) or die "$!: $path_to_delete";
}

my $modep_pedal_dir = $ENV{MODEP_PEDALS} or die "No MODEP_PEDALS defined";
-d $modep_pedal_dir or die "'$modep_pedal_dir' not a directory";
-r $modep_pedal_dir or die "'$modep_pedal_dir' not readable";

## Get the pedal board definitions
my @fn = map{chomp;$_}
grep {$_ !~ /manifest.ttl$/}
grep{/\.ttl$/}
`find $modep_pedal_dir -type f`;

## Each effect is uniquely identified bu `$index`  
my $index = 1;

my @add = ();
my @param = ();
my @jack_init = ();
my %jack_activation = ();

foreach my $fn (@fn){

    my %ex = &process_lv2_turtle($fn, $index);
    $index = $ex{index};

    my $board_name = $ex{pedal_board_name};
    # print "> $board_name\n";

    my $effects = $ex{effects};
    my @lv2_names = sort keys %$effects;
    my %number_name = %{$ex{number_name}};

    my  @j_internal_pipes = @{$ex{jack_internal_pipes}};
    my  @j_activation_pipes = @{$ex{jack_activation_pipes}};

    foreach my $name (@lv2_names){
	# print "\t$name\n";
	my $h = $effects->{$name};
	my @k = sort keys %$h;
	my $add = $h->{add} or die "No `add` for $name";
	# print "\t\t$add\n";
	# print "\t\t".join("", map{"\t\t$_\n"} @param)."\n";
	push(@add, $add);
	push(@param, @{$h->{param}});
    }
    $jack_activation{$board_name} = $ex{jack_activation_pipes};
    push(@jack_init, @{$ex{jack_internal_pipes}});
    
    # foreach my $k (sort keys %number_name){
    # 	my $v = $number_name{$k};
    # 	print "$k => $v\n";
    # }
}

## Output to pedal files.
## Output an initialisation file `Initialse` and a filke for each pedal board

my $pedal_init_fn = "$pedal_dir/Initialise";
open(my $initfh, ">$pedal_init_fn") or die "$!";

## mod-host commands prefixed with "mh"
print $initfh map{"mh $_\n"} @add;
print $initfh map{"mh $_\n"} @param;

## Jack pipes prefixed with "jack"
print $initfh map{"jack $_\n"} @jack_init;
close $initfh or die "$!";
warn "Written $pedal_init_fn\n";
## The activation data.  Pedals use this
foreach my $name (sort keys %jack_activation){
    open(my $actfh, ">$pedal_dir/$name") or die "$!";
    print $actfh map {"$_\n"}
    map{
	## Repair a special case
	/^(capture_\d+):(playback_\d+)/ and $_ = "system:$1 system:$2";
	$_
    }
    @{$jack_activation{$name}};
}


## Passed a file name and a start index, returns a HASH ref that
## describes all the actions required to instantiate a pedal board.
## The `index` is used to identify each effect.  This function is
## called for all the pedal boards at the same time, and each one must
## be independent.  So by initialising the index in the arguments,
## each effect, in a pedal board, across all pedal boards, can have a
## unique index
sub process_lv2_turtle( $$ ) {
    my $fn = shift or die;
    my $index = shift or die; ## Zero is invalid index
    $fn =~ /([^\/]+).ttl$/ or die $fn;
    my $pedal_board_name = $1;

    ## Break up an effect name and port.  This we do a lot
    my $name_port = sub {
        my $name_port = shift or die;
        if($name_port =~ /^(\S+)\/(\S+)/){
            return [$1, $2];
        }else{
            print "$pedal_board_name bad: $name_port\n";
            return undef;
        }
    };

    ## Strip angle brackets from around a value.  We do this a lot as
    ## it turns out
    my $strip_ang = sub {
        my $v = shift or die;
        $v =~ s/^<//;
        $v =~ s/>$//;
        $v
    };

    unless( -r $fn ){
        return ();
    }

    ## Decode the Turtle file
    my @lines = read_turtle($fn) or die "Cannot process $fn";

    ## We need to get the instructions needed to initialise this
    ## effect and turn it on.

    ## Need: 

    ## add <lv2_uri> <instance_number> Record what instance number
    ## goes with what effect so it can be communicated to the user.  

    ## param_set <instance_number> <param_symbol> <param_value>
    ## Set up the effect in the way it was saved from mod-ui

    ## Triples and their meanings
    ## predicate == "lv2:prototype" => subject is an effect, objecty is the URL.
    ## ......... <DS1> lv2:prototype <http://moddevices.com/plugins/mod-devel/DS1> 
    ## _________ Use for the "add" command
    ## predicate == ingen:arc => object names a Jack connection.
    ## .........   "<> ingen:arc _:b1"
    ## _________  Use in "jack_connect" commands
    ## predicate == lv2:port => subject is a device and object is a port of that device
    ## ......... <DS1> lv2:port  <DS1/Out1>
    ## ......... <DS1> lv2:port  <DS1/Tone>

    ## predicate == ingen:tail => subject is a Jack connection, object is where it starts
    ## predicate == ingen:head => subject is a Jack connection, object is where it ends
    ## .........  "_b2 ingen:tail <bitta/output>" 
    ## .........  "_b1 ingen:head <playback_1>
    ## predicate == a  => subject is of type object
    ## .........  <DS1/In> a lv2:AudioPort
    ## .........  <DS1/In> a lv2:InputPort
    ## _________  Use in "jack_connect" commands
    ## .........  <bitta/drywet> a lv2:ControlPort
    ## _________  Use in "param" commands
    ## predicate == "ingen:value" and subject == a control port of a device => object is a value to set a port
    ## .........  <bitta/drywet> ingen:value 1.000000
    ## _________  Use for the "param" command

    ## .........  
    ## .........  

    ## Each effect is setup in this hash.
    ## Indexed by the name	
    my %effects = ();

    ## The internal pipes between the effects that make up the pedal
    ## board and the output.  These are established at startup for all
    ## effects
    my @persistant_jack_pipes = ();

    ## The input audio pipes, and output.  Connecting these enables the effect
    ## chain that makes up the pedal board.  (TODO: What about MIDI
    ## LV2 effects?)
    my @activation_jack_pipes = ();
    
    ## each entry om @line is a triple as text.  Convert into an
    ## array of arrays, each with three elements: subject, predicate,
    ## object
    my @triples = map {
        chomp;
        /^(\S+)\s+(\S+)\s+(.+)/ or die $_;
        [$1, $2, $3]
    } @lines;

    ## Get the commands to add
    my @prototypes = grep {$_->[1] eq "lv2:prototype" } @triples;

    # To map numbers names
    my %name_number = ();
    my %number_name = ();
    
    foreach my $prototype (@prototypes){
        my ($name, $predicate, $uri) = @$prototype;

        ## The name and uri are in angle brackets
        $name = &$strip_ang($name);
        $uri = &$strip_ang($uri);

        $predicate eq "lv2:prototype" or die "Error in prototypes: $predicate";

        ## Initialise the effect hash 
        $effects{$name} = {};
        $effects{$name}->{param} = [];
        $effects{$name}->{add} = "add $uri $index";
        $name_number{$name} = $index;
        $number_name{$index} = $name;

	my @param_set = map{/^<$name\/([^>]+)> ingen:value (.+)\s*$/; "param_set $index $1 $2" } map{join(' ', @$_)} grep{$_->[0] =~ /^<$name\//} grep { $_->[1] =~ /^ingen:value$/ } @triples;
	push(@{$effects{$name}->{param}}, @param_set);
        $index += 1;
    }

    ## Get all the control ports.  As a hash so it can be used to
    ## identify `ingen:value` commands directed at the control ports
    ## of effects in the pedal board
    my $filter_port = sub {
        ## Filter for the ports wanted and get the name/port from
        ## inside the angle brackets
        my $raw = shift or die;
        $raw =~ /^([a-z0-9_]+\/[a-z0-9_\:]+)$/i or 
            # Not a name/port
            return undef; 
        return $1;
    };

    my %control_ports = map{
        &$strip_ang($_) => 1
    } grep {
        defined
    }map{
        &$filter_port(&$strip_ang($_->[0]))
    }grep {
        $_->[1] eq 'a' && $_->[2] eq 'lv2:ControlPort'
    } @triples;

    ## Get all the values for control ports
    my %control_port_values = map {
        &$strip_ang($_->[0]) => $_->[2]
    } grep {
        defined($control_ports{&$strip_ang($_->[0])})
    }grep{
        $_->[1] eq 'ingen:value'
    }grep{
        ## These are some sort of global setting
        ## TODO: Document
        $_->[0] !~ /^:/
    }@triples;

    # ## Set up the `param set` commands in effects
    foreach my $port (keys %control_port_values){
        my $value = $control_port_values{$port};
        $port =~ /([a-z_0-9]+)\/([\:a-z0-9_]+)/i or 
            die "Badly formed port: $port";
        my $name = $1;
        my $port = $2;
        my $number = $name_number{$name};
        defined($number) or die "Unknown name: $name";
        my $command = "param_set $number $port $value";
        push(@{$effects{$name}->{param}}, $command);
    }

    ## Build jack connections
    my @jack_pipes = grep{
        $_->[1] =~ /^ingen:tail$/ or
        $_->[1] =~ /^ingen:head$/ 
    }@triples;

    # There are two sorts of pipe: Internal pipes between effects, and
    # to output, are created at startup.  Activation pipes, pipes from
    # input (capture_N) to first effect in chain 
    my @jack_internal_pipes = ();
    my @jack_activation_pipes = ();
    
    foreach my $pipe (@jack_pipes){
        # `$pipe` is the name of the pipe.  The subject of the triple

        # Get the subject, predicate, and object for both ends of the pipe
        my @records = map {
            [$_->[0], $_->[1], &$strip_ang($_->[2])]
        } grep {
            # Filter by name
            $_->[0] eq $pipe->[0]
                # ## Do not implement MIDI yet.  MIDI pipes eq
                # ## 'midi_merger_out' for now, the only one I have
                # ## seen.  TODO: Make some more pedal boards with MIDI
                # ## controls and watch this die here
                # and $->[2] ne 'midi_merger_out'
        }@triples;

	## Filter out some mysterious MIDI records
        join("", map{$_->[2]} @records) =~ /midi_merger_out/ and next;
        join("", map{$_->[2]} @records) =~ /midi_capture_2/ and next;

        # One "ingen:tail" and one "ingen:head"
        scalar(@records) == 2 or die "Pipe is bad";

        my @tail = grep {$_->[1] eq "ingen:tail"} @records;
        scalar @tail == 1 or  die "Pipe is bad";

        my @head = grep {$_->[1] eq "ingen:head"} @records;
        scalar @head == 1 or  die "Pipe is bad";

        # Activation connections are connected to system:capture_N
        if($tail[0]->[2] =~ /^capture_\d+$/ and 
	       $head[0]->[2] =~ /^playback_\d+$/){
            ## A connection directly from capture to playback
            push(@jack_activation_pipes, "$tail[0]->[2]:$head[0]->[2]");
            next;
        }elsif($tail[0]->[2] =~ /^capture_\d+$/ ){
            ## A connection from the system input
            my $name_port = &$name_port($head[0]->[2]) or die;
            my $number = $name_number{$name_port->[0]};
            my $p = "system:$tail[0]->[2] effect_$number:$name_port->[1]";
            push(@jack_activation_pipes, $p);
            next;
        }elsif($head[0]->[2] =~ /^playback_\d+$/){
            # Output pipe.  An internal pipe
            my $name_port = &$name_port($tail[0]->[2]) or die;
            my $number = $name_number{$name_port->[0]};
            my $p = "effect_$number:$name_port->[1] system:$head[0]->[2]";
            push(@jack_activation_pipes, $p);
            next;
        }

        ## This is an internal pipe
        my $lhs_name_port = &$name_port($tail[0]->[2]) or die;
        my $lhs = "effect_".$name_number{$lhs_name_port->[0]}.":".
            $lhs_name_port->[1];
        my $rhs_name_port = &$name_port($head[0]->[2]) or die;
        my $rhs = "effect_".$name_number{$rhs_name_port->[0]}.":".
            $rhs_name_port->[1];
        my $p = "$lhs $rhs";
        push(@jack_internal_pipes, $p);
    }

    ## Remove duplicates from the jack pipes and parameter definitions
    my %d = ();
    %d = map{$_ => 1} @jack_internal_pipes;
    @jack_internal_pipes = keys %d;
    %d = map{$_ => 1} @jack_activation_pipes;
    @jack_activation_pipes = keys %d;
    foreach my $name (keys %effects){
	%d = map{$_ => 1} @{$effects{$name}->{param}};
	@{$effects{$name}->{param}} = keys %d;
    }
    
    my %result = (
        "effects" => \%effects,
        "index" => $index,
        "jack_activation_pipes" => \@jack_activation_pipes,
        "jack_internal_pipes" => \@jack_internal_pipes,
        "number_name" => \%number_name,
        "pedal_board_name" => $pedal_board_name
        );
    return %result;
    
}
